use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use keys::Direction::*;
use keys::*;
use tty::Terminal;
use utils::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Exit;

#[derive(Debug)]
pub struct Editor {
    // Cursor position (in file)
    cx: u32,
    cy: u32,
    // Offset (window scrolling)
    ox: u32,
    oy: u32,

    term: Terminal,
    lines: Vec<String>,
}

impl Editor {
    pub fn new(term: Terminal) -> Editor {
        Editor {
            cx: 0,
            cy: 0,
            ox: 0,
            oy: 0,
            term: term,
            lines: Vec::new(),
        }
    }

    pub fn init(&mut self) -> io::Result<()> {
        self.term.enable_raw_mode()?;
        self.term.update_window_size()?;
        self.term.clear_screen()?;
        self.refresh_screen()?;
        Ok(())
    }

    pub fn update(&mut self) -> io::Result<Option<Exit>> {
        self.term.update_window_size()?;

        let km = self.term.get_key()?;
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
            Command::Nothing => (),
            Command::Exit => {
                self.refresh_screen()?;
                return Ok(Some(Exit));
            }
            Command::InsertCharacter(ch) => {
                eprintln!("typing: {}", ch);
            }
            Command::Move(d) => match d {
                Direction::Left => {
                    if self.cx > 0 {
                        self.cx -= 1
                    }
                }
                Direction::Right => self.cx += 1,
                Direction::Up => {
                    if self.cy > 0 {
                        self.cy -= 1
                    }
                }
                Direction::Down => self.cy += 1,
            },
            Command::MovePageUp => {
                self.cx = 0;
                self.cy = 0;
            }
            Command::MovePageDown => {
                self.cx = 0;
                self.cy = self.term.wy - 1;
            }
            Command::MoveLineHome => {
                self.cx = 0;
            }
            Command::MoveLineEnd => {
                self.cx = self.term.wx - 1;
            }
            Command::Erase(Left) => {
                if self.cx > 0 {
                    self.cx -= 1
                }
            }
            Command::Erase(Right) => self.cx += 1,
            Command::Erase(_) => (),
        }
        // Scroll to show cursor
        self.ox = 0;
        while (self.cx - self.ox) >= (self.term.wx as f32 * 0.90) as u32 {
            self.ox += (self.term.wx as f32 * 0.85) as u32;
        }
        self.refresh_screen().expect("refresh_screen");
        eprintln!(
            "editor: {{ cx: {}, cy: {}, ox: {}, oy: {} }}",
            self.cx, self.cy, self.ox, self.oy
        );
        Ok(None)
    }

    pub fn deinit(&mut self) -> io::Result<()> {
        self.term.disable_raw_mode()
    }

    pub fn open(&mut self, fname: &Path) -> io::Result<()> {
        let file = io::BufReader::new(File::open(fname)?);
        for line in file.lines().map(|l| l.unwrap()) {
            self.lines.push(line);
        }
        Ok(())
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        if self.cy < self.oy {
            self.oy = self.cy;
        }
        if self.cx < self.ox {
            self.ox = self.cx;
        }
        if self.cy >= self.oy + self.term.wy {
            self.oy = self.cy - self.term.wy + 1;
        }
        if self.cx >= self.ox + self.term.wx {
            self.ox = self.cx - self.term.wx + 1;
        }

        let status = format!("? ~~  rk v{}  ~~", VERSION);
        self.term.hide_cursor()?;
        self.term.move_cursor_topleft()?;
        for y in 0..(self.term.wy - 1) {
            let filerow = (y as u32 + self.oy) as usize;
            self.term.clear_line()?;
            if filerow < self.lines.len() {
                let line = &self.lines[filerow];
                let line = line.uslice(
                    self.ox as usize,
                    (self.ox + self.term.wx) as usize,
                );
                self.term.write(line.as_bytes())?;
            } else {
                self.term.write(b"~")?;
            }
            self.term.write(b"\r\n")?;
        }
        self.term.write(status.as_bytes())?;
        self.term
            .move_cursor(self.cx - self.ox, self.cy - self.oy)?;
        self.term.show_cursor()?;
        self.term.flush()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Command {
    Nothing,
    InsertCharacter(char),
    Move(Direction),
    MovePageUp,
    MovePageDown,
    MoveLineHome,
    MoveLineEnd,
    Erase(Direction),
    Exit,
}
