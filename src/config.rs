use crate::{Mode, Command, Keybind, State};
use crossterm::terminal;
use std::{fs::File, io::Write};

fn insert_keybinds() -> Vec<Keybind> {
    vec![
    Keybind {
        mode: Mode::Insert,
        key: "<any>",
        function: |state, c| {
            state.text[state.file_posy].insert(state.file_posx , c.chars().next().unwrap()); 
            state.file_posx += 1;
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Insert,
        key: "<cr>",
        function: |state, _| {
            let new = state.text[state.file_posy].split_off(state.file_posx);
            state.text.insert(state.file_posy+1, new);
            state.file_posx = 0;
            state.file_posy += 1; 
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Insert,
        key: "<back>",
        function: |state, _| {
            if state.file_posx != 0 {
                state.file_posx -= 1;
                state.text[state.file_posy].remove(state.file_posx);
            }
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Insert,
        key: "<esc>",
        function: |state, _| {
            state.mode = Mode::Normal;
            Ok(())
        },
    },
    ]
}
fn normal_keybinds() -> Vec<Keybind> {

    vec![Keybind {
        mode: Mode::Normal, 
        key: "i", 
        function: |state, _| {
            state.mode = Mode::Insert;
            Ok(())
        }
    },
    Keybind {
        mode: Mode::Normal, 
        key: "q", 
        function: |_, _| {
            terminal::disable_raw_mode()?;
            std::process::exit(0);
        }
    },
    Keybind {
        mode: Mode::Normal, 
        key: "w", 
        function: |state, _| {
            if let Some(filename) = &state.filename {
                let mut file = File::create(filename)?;
                file.write_all(state.to_string("\n").as_bytes())?;
            }
            Ok(())
        }
    },
    Keybind {
        mode: Mode::Normal, 
        key: "j", 
        function: |state, _| {
            if state.file_posy != state.text.len() - 1 {
                state.file_posy += 1;
                state.update_x(state.file_posx);
            }
            Ok(())
        }
    },
    Keybind {
        mode: Mode::Normal,
        key: "k",
        function: |state, _| {
            if state.file_posy != 0 {
                state.file_posy -= 1;
                state.update_x(state.file_posx);
            }
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Normal,
        key: "h",
        function: |state, _| {
            if state.file_posx != 0 {
                state.update_x(state.file_posx-1);
            }
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Normal,
        key: "l",
        function: |state, _| {
            state.update_x(state.file_posx+1);
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Normal,
        key: ":",
        function: |state, _| {
            state.mode = Mode::Command;
            Ok(())
        },
    },
    ]
}

fn clear_command(state: &mut State) {
    state.prompbuffer.clear();
    state.mode = Mode::Normal;
}

fn command_keybinds() -> Vec<Keybind> {
    vec![
    Keybind {
        mode: Mode::Command,
        key: "<esc>",
        function: |state, _| {
            clear_command(state);
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Command,
        key: "<any>",
        function: |state, c| {
            state.prompbuffer.push_str(&c);
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Command,
        key: "<back>",
        function: |state, _| {
            state.prompbuffer.pop();
            Ok(())
        },
    },
    Keybind {
        mode: Mode::Command,
        key: "<cr>",
        function: |state, _| {
            state.execute_command()?;
            clear_command(state);
            Ok(())
        },
    },
    ]
}

pub fn get_commands() -> Vec<Command> {
    vec![
        Command {
            name: "q",
            function: |_| {
                terminal::disable_raw_mode()?;
                std::process::exit(0);
            },
        }
    ]
}
pub fn get_keybinds() -> Vec<Keybind> {
    let mut result = insert_keybinds();
    result.extend(command_keybinds());
    result.extend(normal_keybinds());
    result
}
