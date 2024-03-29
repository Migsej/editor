use std::{fs::{self, File}, io::{self, Write, Error, Stdout}, env};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style, event::{read, Event, KeyCode}
};

mod config;
use crate::config::{get_keybinds, get_commands};

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
    Command,
}

struct Visualline {
    line: usize,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
pub struct Keybind {
    mode: Mode,
    key: &'static str,
    function: fn(&mut State, String) -> io::Result<()>,
}

pub struct Command {
    name: &'static str,
    function: fn(&mut State) -> io::Result<()>,
}
struct Patch {
    c: char,
    color: style::Color,
}

struct State {
    stdout: Stdout,

    text: Vec<String>,
    visualtext: Vec<Visualline>,
    viewport: Vec<Vec<Patch>>,
    oldport: Vec<Vec<Patch>>,

    cursor_posx: u16,
    cursor_posy: u16,
    file_posx: usize,
    file_posy: usize,
    old_posx: usize,
    old_posy: usize,

    height: u16,
    width: u16,

    mode: Mode,

    filename: Option<String>,
    keybinds: Vec<Keybind>,
    commands: Vec<Command>,

    prompbuffer: String,
}

impl State  {
    fn new(startext: Vec<String>, width: u16, height: u16) -> State {
        State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: None, commands: crate::get_commands(), keybinds: crate::get_keybinds(), prompbuffer: String::new(), stdout: io::stdout(), oldport: Vec::new(), viewport: Vec::new()}
    }
    fn new_from_file(file: String, width: u16, height: u16) -> io::Result<State> {
        let contents = fs::read_to_string(&file).or_else(|_| {
            File::create(&file)?;
            return Ok::<String, Error>(String::from(""))
        })?;
        let startext = contents.split('\n').map(|x| x.to_string()).collect();
        Ok( State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: Some(file), commands: crate::get_commands(), keybinds: crate::get_keybinds(), prompbuffer: String::new(), stdout: io::stdout(), oldport: Vec::new(), viewport: Vec::new()})
    }

    fn to_string(&self, sep: &str) -> String {
        self.text.join(sep)
    }

    fn update_x(&mut self, new_x: usize) {
        self.file_posx = std::cmp::max(0, std::cmp::min(self.text[self.file_posy].len(), new_x))
    }

    fn find_visual_line(&self, line: usize) -> usize {
        let mut i = line;
        while self.visualtext[i].line != line {
            i += 1;
        }
        return i;

    }
    fn get_view_port(&self) -> String {
        let visualpositiony = self.find_visual_line(self.file_posy);
        let upper = ((visualpositiony as isize - self.cursor_posy as isize) + self.file_posx as isize / self.width as isize) as usize;
        let lower = std::cmp::min(self.visualtext.len(), upper + self.height as usize);
        let mut result =  self.visualtext[upper..lower]
            .iter()
            .map(|x| self.text[x.line][x.start..x.end].to_string())
            .collect::<Vec<_>>();
        if self.mode == Mode::Command {
            result.truncate(self.height as usize -1);
        }
        return result.join("\r\n");
    }

    fn update_cursor(&mut self) {
        let visual_newy = self.find_visual_line(self.file_posy) + self.file_posx / self.width as usize;
        let visual_oldy = self.find_visual_line(self.old_posy) + self.old_posx / self.width as usize;
        let new_cursorposy = self.cursor_posy as isize 
                           + (visual_newy as isize - visual_oldy as isize);

        if new_cursorposy < self.height as isize && new_cursorposy >= 0  {
            self.cursor_posy = new_cursorposy as u16 ;
        }

        self.cursor_posx = self.file_posx as u16 % self.width;
    }

    fn update_visuallines(&mut self) {
        //TODO: perfomance
        self.visualtext.clear();
        for (line_num, line) in self.text.iter().enumerate() {
            let mut start = 0;
            let end = line.len();
            while end - start > self.width as usize {
                self.visualtext.push(Visualline { line: line_num, start, end: start + self.width as usize });
                start += self.width as usize;
            }
            self.visualtext.push(Visualline { line: line_num, start, end});
        }
    }
    fn execute_keybind(&mut self, keybinds: Vec<(Keybind, String)>) -> io::Result<()> {
        match read()? {
            Event::Resize(width, height) => {
                self.width = width;
                self.height = height;
            },
            Event::Key(event) => {
                let mut key: String = String::new();
                if let KeyCode::Char(c) = event.code {
                    key.push(c)
                } else {
                    key.push_str(match event.code {
                        KeyCode::Char(_) => unreachable!(),
                        KeyCode::Enter => "<cr>",
                        KeyCode::Backspace => "<back>",
                        KeyCode::Esc => "<esc>",
                        _ => "",
                    });
                }
                let filtered: Vec<(Keybind, String, &str)> = keybinds
                    .into_iter()
                    .filter_map(|(keybind, mut pressed)| {
                        let first_keybind = keybind.key;
                        if keybind.key[pressed.len()..].starts_with(&key) &&  keybind.mode == self.mode {
                            pressed.push_str(&key); // might be useful
                            Some((keybind.clone(), pressed, first_keybind))
                        } else if keybind.key[pressed.len()..].starts_with("<any") &&  keybind.mode == self.mode {
                            pressed.push_str(&key);
                            return Some((keybind.clone(), pressed, first_keybind));
                        } else {
                            return None
                        }
                    }).collect();
                let length = filtered.len();
                if length == 0 {
                    return Ok(())
                }
                let notany: Vec<_> = filtered.iter().filter(|x| !x.0.key.ends_with("<any>")).collect();
                if  notany.len() == 1 {
                    let head = &notany[0];
                    if length_of_keybind(head.0.key) == length_of_keybind(&head.1){
                        return (head.0.function)(self, head.1.to_string());
                    }
                }
                if length == 1 {
                    let head = &filtered[0];
                    if length_of_keybind(head.0.key) == length_of_keybind(&head.1){
                        return (head.0.function)(self, head.1.to_string());
                    }
                }
                return self.execute_keybind(filtered.into_iter().map(|(keybind, pressed, _)| (keybind, pressed)).collect());
            },
            _ => (),
        }
        Ok(())
    }
    
    fn draw_command_prompt(&mut self) -> std::io::Result<()> {
        if self.mode != Mode::Command {
            return Ok(())
        }

        self.stdout.queue(cursor::MoveTo(0, self.height))?
              .queue(style::Print(':'))?
              .queue(style::Print(&self.prompbuffer))?;
        Ok(())
    }
    fn execute_command(&mut self) -> std::io::Result<()> {
        match self.commands.iter().find(|x| x.name == self.prompbuffer) {
            Some(command) => {
                return (command.function)(self);
            },
            None => Ok(()),
        }
    }

    fn quit(&mut self) {
        self.stdout.execute(cursor::SetCursorStyle::BlinkingBlock).unwrap();
        self.stdout.execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
        std::process::exit(0);
    }

    fn write_to_viewport
}

fn length_of_keybind(key: &str) -> usize {
    let mut countup = true;
    let mut result = 0;
    for c in key.chars() {
         if c == '<' {
             countup = false;
         } else if c == '>' {
             result += 1;
             countup = true;
         } else {
             if countup {
                 result += 1;
             }
         }
    }
    result
}


fn main() -> io::Result<()> {
    
    let args: Vec<String> = env::args().collect();

    let mut state: State;

    let (width, height) = terminal::size()?;
    if args.len() > 1 {
        let filename = &args[1];
        state = State::new_from_file(filename.to_string(), width, height)?;
    } else {
        let startext = vec![String::new()];
        state = State::new(startext, width, height);
    }
    state.stdout.execute(terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    state.stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    state.stdout.execute(cursor::MoveTo(0,0))?;
    
    let defaultkeybinds: Vec<_> = state.keybinds.clone().into_iter().map(|x| (x, String::new())).collect();

    loop {
        let cursorstyle = match state.mode {
            Mode::Normal => cursor::SetCursorStyle::BlinkingBlock,
            Mode::Insert | Mode::Command  => cursor::SetCursorStyle::BlinkingBar,
        };
        

        state.update_visuallines();
        state.update_cursor();

        state.stdout.queue(terminal::Clear(terminal::ClearType::All))?
              .queue(cursorstyle)?
              .queue(cursor::MoveTo(0,0))?;

        state.stdout.queue(style::Print(state.get_view_port()))?;
        state.draw_command_prompt()?;

        if state.mode != Mode::Command {
            state.stdout.queue(cursor::MoveTo(state.cursor_posx, state.cursor_posy))?;
        }

        state.stdout.flush()?;

        state.old_posx = state.file_posx;
        state.old_posy = state.file_posy;

        state.execute_keybind(defaultkeybinds.clone())?;
    }
}
