use std::fs::File;
use std::io::{self, Write};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style::{self}, event::{read, Event, KeyCode}
};


struct State {
    text: Vec<String>,
    cursor_viewport_pos: u16,
    file_posx: usize,
    file_posy: usize,

    height: u16,
    width: u16,
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
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0,0))?;
    
    let (width, height) = terminal::size()?;
    let mut state = State { text: vec![String::new()], cursor_viewport_pos: 0, file_posx: 0, file_posy: 0,  width, height };
    loop {
        match read()? {
            Event::Resize(width, height) => {
                state.width = width;
                state.height = height;
            },
            Event::Key(event) => {
                match event.code {
                    KeyCode::Char(c) => {
                        if 'q' == c {
                            terminal::disable_raw_mode()?;
                            return Ok(());
                        }else if 'w' == c {
                            let mut file = File::create("foo.txt")?;
                            file.write_all(state.to_string("\n").as_bytes())?;
                        } else {
                            state.text[state.file_posy].insert(state.file_posx , c);
                            state.file_posx += 1;
                        }
                    },
                    KeyCode::Enter => {
                        let new = state.text[state.file_posy].split_off(state.file_posx);
                        state.text.insert(state.file_posy+1, new);
                        state.file_posx = 0;
                        state.file_posy += 1; 
                        if state.cursor_viewport_pos < state.height-1 {
                            state.cursor_viewport_pos += 1;
                        }
                    },
                    KeyCode::Down => {
                        if state.file_posy != state.text.len() - 1 {
                            state.file_posy += 1;
                            if state.cursor_viewport_pos < state.height-1 {
                                state.cursor_viewport_pos += 1;
                            }
                            state.update_x(state.file_posx);
                        }
                    },
                    KeyCode::Up => {
                        if state.file_posy != 0 {
                            state.file_posy -= 1;
                            if state.cursor_viewport_pos > 0 {
                                state.cursor_viewport_pos -= 1;
                            }
                            state.update_x(state.file_posx);
                        }
                    },
                    KeyCode::Left => {
                        if state.file_posx != 0 {
                            state.update_x(state.file_posx-1);
                        }
                    },
                    KeyCode::Right => {
                        state.update_x(state.file_posx+1);
                    },
                    KeyCode::Backspace => {
                        state.file_posx -= 1;
                        state.text[state.file_posy].remove(state.file_posx);
                    },
                    _ => (),
                }
            },
            _ => (),
        }
        stdout.queue(terminal::Clear(terminal::ClearType::All))?
              .queue(cursor::MoveTo(0,0))?
              .queue(style::Print(&state.get_viewport()))?
              .queue(cursor::MoveTo(state.file_posx as u16, state.cursor_viewport_pos))?;
        stdout.flush()?;
    }
}
