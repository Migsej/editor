use std::{fs::{self, File}, io::{self, Write}, env};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style, event::{read, Event, KeyCode, KeyEvent}
};

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
}

struct State {
    text: Vec<String>,
    cursor_viewport_pos: u16,
    file_posx: usize,
    file_posy: usize,

    height: u16,
    width: u16,

    mode: Mode,
}


impl State {
    fn to_string(&self, sep: &str) -> String {
        self.text.join(sep)
    }
    fn update_x(&mut self, new_x: usize) {
        self.file_posx = std::cmp::max(0, std::cmp::min(self.text[self.file_posy].len(), new_x))
    }

    fn get_viewport(&self) -> String {
        let upper = std::cmp::max(0, self.file_posy - self.cursor_viewport_pos as usize);
        let lower = std::cmp::min(self.text.len(), self.file_posy + (self.height - self.cursor_viewport_pos) as usize );
        self.text[upper..lower].join("\r\n")
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
                if self.cursor_viewport_pos < self.height-1 {
                    self.cursor_viewport_pos += 1;
                }
            },
            KeyCode::Backspace => {
                self.file_posx -= 1;
                self.text[self.file_posy].remove(self.file_posx);
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
                        let mut file = File::create("foo.txt")?;
                        file.write_all(self.to_string("\n").as_bytes())?;
                    },
                    'j' => {
                        if self.file_posy != self.text.len() - 1 {
                            self.file_posy += 1;
                            if self.cursor_viewport_pos < self.height-1 {
                                self.cursor_viewport_pos += 1;
                            }
                            self.update_x(self.file_posx);
                        }
                    },
                    'k' => {
                        if self.file_posy != 0 {
                            self.file_posy -= 1;
                            if self.cursor_viewport_pos > 0 {
                                self.cursor_viewport_pos -= 1;
                            }
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

    let mut startext = vec![String::new()];
    if args.len() > 1 {
        let filename = &args[1];
        let contents = fs::read_to_string(filename)?;
        startext = contents.split('\n').map(|x| x.to_string()).collect();
    }

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0,0))?;

    
    let (width, height) = terminal::size()?;
    let mut state = State { text: startext, cursor_viewport_pos: 0, file_posx: 0, file_posy: 0,  width, height, mode: Mode::Normal };
    loop {
        let cursorstyle = match state.mode {
            Mode::Normal => cursor::SetCursorStyle::BlinkingBlock,
            Mode::Insert => cursor::SetCursorStyle::BlinkingBar,
        };

        stdout.queue(terminal::Clear(terminal::ClearType::All))?
              .queue(cursor::MoveTo(0,0))?
              .queue(cursorstyle)?
              .queue(style::Print(&state.get_viewport()))?
              .queue(cursor::MoveTo(state.file_posx as u16, state.cursor_viewport_pos))?;
        stdout.flush()?;


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
