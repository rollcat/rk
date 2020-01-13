use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use keys::*;
use tty::Terminal;
use utils::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct Editor {
    // Cursor position (in file)
    pub cx: u32,
    pub cy: u32,
    // Offset (window scrolling)
    pub ox: u32,
    pub oy: u32,

    pub term: Terminal,
    pub lines: Vec<String>,
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
