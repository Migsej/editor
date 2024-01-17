use std::{fs::{self, File}, io::{self, Write, Error}, env};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style, event::{read, Event, KeyCode, KeyEvent}
};

mod config;
use crate::config::get_keybinds;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
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


struct State {
    text: Vec<String>,
    visualtext: Vec<Visualline>,
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
}


impl State  {
    fn new(startext: Vec<String>, width: u16, height: u16) -> State {
        State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: None, keybinds: crate::get_keybinds()}
    }
    fn new_from_file(file: String, width: u16, height: u16) -> io::Result<State> {
        let contents = fs::read_to_string(&file).or_else(|_| {
            File::create(&file)?;
            return Ok::<String, Error>(String::from(""))
        })?;
        let startext = contents.split('\n').map(|x| x.to_string()).collect();
        Ok( State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: Some(file), keybinds: crate::get_keybinds()})
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
        return self.visualtext[upper..lower]
            .iter()
            .map(|x| self.text[x.line][x.start..x.end].to_string())
            .collect::<Vec<_>>()
            .join("\r\n");
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
                    .filter_map(|(mut keybind, mut pressed)| {
                        let first_keybind = keybind.key;
                        if keybind.key.starts_with(&key) &&  keybind.mode == self.mode {
                            keybind.key = &keybind.key[key.len()..];
                            //pressed.push_str(&key); // might be useful
                            Some((keybind.clone(), pressed, first_keybind))
                        } else if keybind.key.starts_with("<any>") &&  keybind.mode == self.mode {
                            keybind.key = &keybind.key["<any>".len()..];
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
                let notany: Vec<_> = filtered.iter().filter(|x| x.2 != "<any>").collect();
                if  notany.len() == 1 {
                    let head = &notany[0];
                    return (head.0.function)(self, head.1.to_string());
                }
                if length == 1 {
                    let head = &filtered[0];
                    return (head.0.function)(self, head.1.to_string());
                }
                return self.execute_keybind(filtered.into_iter().map(|(keybind, pressed, _)| (keybind, pressed)).collect());
            },
            _ => (),
        }
        Ok(())
    }

}


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut state: State;

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0,0))?;

    let (width, height) = terminal::size()?;
    if args.len() > 1 {
        let filename = &args[1];
        state = State::new_from_file(filename.to_string(), width, height)?;
    } else {
        let startext = vec![String::new()];
        state = State::new(startext, width, height);
    }
    
    let defaultkeybinds: Vec<_> = state.keybinds.clone().into_iter().map(|x| (x, String::new())).collect();

    loop {
        let cursorstyle = match state.mode {
            Mode::Normal => cursor::SetCursorStyle::BlinkingBlock,
            Mode::Insert => cursor::SetCursorStyle::BlinkingBar,
        };
        

        state.update_visuallines();
        state.update_cursor();

        stdout.queue(terminal::Clear(terminal::ClearType::All))?
              .queue(cursor::MoveTo(0,0))?
              .queue(cursorstyle)?
              .queue(style::Print(&state.get_view_port()))?
              .queue(cursor::MoveTo(state.cursor_posx, state.cursor_posy))?;
        stdout.flush()?;

        state.old_posx = state.file_posx;
        state.old_posy = state.file_posy;
        state.execute_keybind(defaultkeybinds.clone())?;
    }
}
