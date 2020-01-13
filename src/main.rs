extern crate libc;
extern crate regex;
extern crate termios;

use std::env;
use std::io;
use std::path::Path;

mod editor;
mod keys;
mod tty;
mod utils;

use editor::{Command, Editor};
use keys::Direction::*;
use keys::*;
use tty::Terminal;

fn main() {
    let args: Vec<String> = env::args().collect();
    let t = Terminal::new(io::stdin(), io::stdout());
    let mut e = Editor::new(t);

    if args.len() == 2 {
        let path = Path::new(&args[1]);
        e.open(path).expect("open");
    }

    e.term.enable_raw_mode().expect("could not enable raw mode");
    e.term.update_window_size().expect("update_window_size");
    e.term.clear_screen().expect("clear_screen");
    e.refresh_screen().expect("refresh_screen");
    loop {
        e.term.update_window_size().expect("update_window_size");

        let km = e.term.get_key().expect("get_key");
        eprintln!("key: {:?}", km);
        let KeyMod {
            key,
            ctrl,
            meta,
            shift,
        } = km;
        let cmd = match (ctrl, meta, shift, key) {
            (true, false, false, Key::Char('q')) => Command::Exit,
            (false, false, _, Key::Direction(d)) => Command::Move(d),
            (true, false, _, Key::Direction(d)) => match d {
                Direction::Up => Command::MovePageUp,
                Direction::Down => Command::MovePageDown,
                Direction::Left => Command::MoveLineHome,
                Direction::Right => Command::MoveLineEnd,
            },
            (false, false, _, Key::Char(ch)) => Command::InsertCharacter(ch),
            (false, false, _, Key::PageUp) => Command::MovePageUp,
            (false, false, _, Key::PageDown) => Command::MovePageDown,
            (false, false, _, Key::Home) => Command::MoveLineHome,
            (false, false, _, Key::End) => Command::MoveLineEnd,
            (false, false, _, Key::Backspace) => Command::Erase(Left),
            (false, false, _, Key::Delete) => Command::Erase(Right),
            (_, _, _, key) => {
                eprintln!(
                    "unhandled: {:?}",
                    KeyMod {
                        key,
                        ctrl,
                        meta,
                        shift,
                    }
                );
                Command::Nothing
            }
        };

        match cmd {
            Command::Nothing => continue,
            Command::Exit => {
                e.refresh_screen().expect("refresh_screen");
                break;
            }
            Command::InsertCharacter(ch) => {
                eprintln!("typing: {}", ch);
            }
            Command::Move(d) => match d {
                Direction::Left => {
                    if e.cx > 0 {
                        e.cx -= 1
                    }
                }
                Direction::Right => e.cx += 1,
                Direction::Up => {
                    if e.cy > 0 {
                        e.cy -= 1
                    }
                }
                Direction::Down => e.cy += 1,
            },
            Command::MovePageUp => {
                e.cx = 0;
                e.cy = 0;
            }
            Command::MovePageDown => {
                e.cx = 0;
                e.cy = e.term.wy - 1;
            }
            Command::MoveLineHome => {
                e.cx = 0;
            }
            Command::MoveLineEnd => {
                e.cx = e.term.wx - 1;
            }
            Command::Erase(Left) => {
                if e.cx > 0 {
                    e.cx -= 1
                }
            }
            Command::Erase(Right) => e.cx += 1,
            Command::Erase(_) => (),
        }
        // Scroll to show cursor
        e.ox = 0;
        while (e.cx - e.ox) >= (e.term.wx as f32 * 0.90) as u32 {
            e.ox += (e.term.wx as f32 * 0.85) as u32;
        }
        e.refresh_screen().expect("refresh_screen");
        eprintln!(
            "editor: [ cx: {}, cy: {}, ox: {}, oy: {} ]",
            e.cx, e.cy, e.ox, e.oy
        );
    }

    e.term
        .disable_raw_mode()
        .expect("could not disable raw mode");
    eprintln!("e: {:?}", e);
}
