use std::{fs::{self, File}, io::{self, Write, Error}, env};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style, event::{read, Event, KeyCode, KeyEvent}
};

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
}

struct Visualline {
    line: usize,
    start: usize,
    end: usize,
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
}


impl State  {
    fn new(startext: Vec<String>, width: u16, height: u16) -> State {
        State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: None }
    }
    fn new_from_file(file: String, width: u16, height: u16) -> io::Result<State> {
        let contents = fs::read_to_string(&file).or_else(|_| {
            File::create(&file)?;
            return Ok::<String, Error>(String::from(""))
        })?;
        let startext = contents.split('\n').map(|x| x.to_string()).collect();
        Ok( State { text: startext, cursor_posx: 0, cursor_posy: 0, old_posx: 0, old_posy: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal, visualtext: Vec::new(), filename: Some(file) })
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

    fn insertmodeinput(&mut self, event: KeyEvent) -> io::Result<()> {
        match event.code {
            KeyCode::Char(c) => {
                self.text[self.file_posy].insert(self.file_posx , c);
                self.file_posx += 1;
            },
            KeyCode::Enter => {
                let new = self.text[self.file_posy].split_off(self.file_posx);
                self.text.insert(self.file_posy+1, new);
                self.file_posx = 0;
                self.file_posy += 1; 
            },
            KeyCode::Backspace => {
                if self.file_posx != 0 {
                    self.file_posx -= 1;
                    self.text[self.file_posy].remove(self.file_posx);
                }
            },
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            },
            _ => (),
        }
        Ok(())
    }

    fn normalmodeinput(&mut self, event: KeyEvent) -> io::Result<()> {
        match event.code {
            KeyCode::Char(c) => {
                match c {
                    'q' => {
                        terminal::disable_raw_mode()?;
                        std::process::exit(0);
                    },
                    'w' => {
                        if let Some(filename) = &self.filename {
                            let mut file = File::create(filename)?;
                            file.write_all(self.to_string("\n").as_bytes())?;
                        }
                    },
                    'j' => {
                        if self.file_posy != self.text.len() - 1 {
                            self.file_posy += 1;
                            self.update_x(self.file_posx);
                        }
                    },
                    'k' => {
                        if self.file_posy != 0 {
                            self.file_posy -= 1;
                            self.update_x(self.file_posx);
                        }
                    },
                    'h' => {
                        if self.file_posx != 0 {
                            self.update_x(self.file_posx-1);
                        }
                    },
                    'l' => {
                        self.update_x(self.file_posx+1);
                    },
                    'i' => {
                        self.mode = Mode::Insert;
                    },
                    _ => (),
                }
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

        match read()? {
            Event::Resize(width, height) => {
                state.width = width;
                state.height = height;
            },
            Event::Key(event) => {
                if state.mode == Mode::Normal {
                    state.normalmodeinput(event)?;
                } else if state.mode == Mode::Insert {
                    state.insertmodeinput(event)?;
                }
            },
            _ => (),
        }
    }
}
