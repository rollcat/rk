use std::cmp::min;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use termion::event::{Event, Key};


use tty;
use utils::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Exit;

#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub struct Editor {
    // Frontend
    term: tty::Terminal,

    // Buffer / window with active file
    fname: String,
    lines: Vec<String>,
    // Cursor position (in file)
    cx: usize,
    cy: usize,
    // Offset (window scrolling)
    ox: usize,
    oy: usize,

    // Status line
    message: String,
}

impl Editor {
    pub fn new(term: tty::Terminal) -> Editor {
        Editor {
            cx: 0,
            cy: 0,
            ox: 0,
            oy: 0,
            term: term,
            lines: Vec::new(),
            message: String::new(),
            fname: String::from("*scratch*"),
        }
    }

    pub fn init(&mut self) -> io::Result<()> {
        self.term.update()?;
        self.term.clear_screen()?;
        self.update_screen()?;
        Ok(())
    }

    pub fn update(&mut self) -> io::Result<Option<Exit>> {
        self.term.update()?;

        let cmd = self.update_input()?;
        let status = self.exec_cmd(cmd)?;
        self.scroll_to_cursor();
        self.update_screen()?;
        Ok(status)
    }

    pub fn deinit(&mut self) -> io::Result<()> {
        write!(
            self.term,
            "{}{}{}",
            termion::clear::All,
            termion::style::Reset,
            termion::cursor::Goto(1, 1)
        )?;
        Ok(())
    }

    pub fn open(&mut self, fname: &Path) -> io::Result<()> {
        let file = io::BufReader::new(File::open(fname)?);
        for line in file.lines().map(|l| l.unwrap()) {
            self.lines.push(line);
        }
        if let Some(bname) = fname.file_name() {
            self.fname = String::from(bname.to_str().unwrap());
        }
        Ok(())
    }

    fn update_input(&mut self) -> io::Result<Command> {
        let ev = self.term.get_event()?;
        self.message = String::from(format!("rk v{}", VERSION));
        let cmd = match ev {
            Event::Key(k) => match k {
                Key::Ctrl('q') => Command::Exit,
                Key::Alt('Q') => panic!("forced a panic"),
                Key::Up => Command::Move(Direction::Up),
                Key::Down => Command::Move(Direction::Down),
                Key::Left => Command::Move(Direction::Left),
                Key::Right => Command::Move(Direction::Right),
                Key::Char(ch) => Command::InsertCharacter(ch),
                Key::Ctrl('m') => Command::InsertCharacter('\n'),
                Key::PageUp => Command::MovePageUp,
                Key::PageDown => Command::MovePageDown,
                Key::Home => Command::MoveLineHome,
                Key::End => Command::MoveLineEnd,
                Key::Backspace => Command::Erase(Direction::Left),
                Key::Delete => Command::Erase(Direction::Right),
                _ => {
                    self.message = format!("key not bound: {:?}", ev,);
                    Command::Nothing
                }
            },
            _ => Command::Nothing,
        };
        Ok(cmd)
    }

    fn exec_cmd(&mut self, cmd: Command) -> io::Result<Option<Exit>> {
        match cmd {
            Command::Nothing => (),
            Command::Exit => {
                self.update_screen()?;
                return Ok(Some(Exit));
            }
            Command::InsertCharacter(ch) => {
                self.exec_cmd_insert(ch);
            }
            Command::Move(d) => {
                self.exec_cmd_move(d);
            }
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
            Command::Erase(d) => {
                self.exec_cmd_erase(d);
            }
        }
        return Ok(None);
    }

    fn exec_cmd_move(&mut self, d: Direction) {
        match d {
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
        }
        self.cx = min(self.cx, self.lines[self.cy].len());
    }

    fn exec_cmd_insert(&mut self, ch: char) {
        match ch {
            '\n' => {
                // Take the current line, and break it in two
                let line = self.lines[self.cy].clone();
                self.lines[self.cy] = line.uslice(0, self.cx);
                self.cy += 1;
                self.lines.insert(self.cy, line.uslice(self.cx, line.len()));
                self.cx = 0;
            }
            _ => {
                // Insert in the middle of the current line
                let line = &self.lines[self.cy];
                let len = line.len();
                let mut newline = String::with_capacity(len + 1);
                newline.push_str(&line.uslice(0, self.cx));
                newline.push(ch);
                newline.push_str(&line.uslice(self.cx, len));
                self.lines[self.cy] = newline;
                self.exec_cmd_move(Direction::Right);
            }
        }
    }

    fn exec_cmd_erase(&mut self, d: Direction) {
        match d {
            Direction::Left => {
                if self.cx == 0 {
                    if self.cy == 0 {
                        // top left, do nothing
                        return;
                    }
                    // join this line with previous
                    let line = self.lines.remove(self.cy);
                    self.cy -= 1;
                    self.cx = self.lines[self.cy].len();
                    let mut newline =
                        String::with_capacity(self.cx + line.len());
                    newline.push_str(&self.lines[self.cy]);
                    newline.push_str(&line);
                    self.lines[self.cy] = newline;
                } else {
                    // remove from the middle
                    let line = &self.lines[self.cy];
                    let len = line.len();
                    let mut newline = String::with_capacity(len - 1);
                    newline.push_str(&line.uslice(0, self.cx - 1));
                    newline.push_str(&line.uslice(self.cx, len));
                    self.lines[self.cy] = newline;
                    self.exec_cmd_move(Direction::Left);
                }
            }
            Direction::Right => {
                // todo
            }
            _ => {} // noop
        }
    }

    fn scroll_to_cursor(&mut self) {
        self.ox = 0;
        while (self.cx - self.ox) >= (self.term.wx as f32 * 0.90) as usize {
            self.ox += (self.term.wx as f32 * 0.85) as usize;
        }
    }

    fn update_screen(&mut self) -> io::Result<()> {
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

        let status = format!(
            "? {fname} {line}:{col} -- {message}",
            fname = self.fname,
            col = self.cx,
            line = self.cy + 1,
            message = self.message,
        );
        self.term.hide_cursor()?;
        self.term.move_cursor_topleft()?;
        for y in 0..(self.term.wy - 1) {
            let filerow = y + self.oy;
            self.term.clear_line()?;
            if filerow < self.lines.len() {
                let line = &self.lines[filerow];
                let line = line.uslice(self.ox, self.ox + self.term.wx);
                self.term.write(line.as_bytes())?;
            } else {
                self.term.write(b"~")?;
            }
            self.term.write(b"\r\n")?;
        }
        self.term.write(status.uslice(0, self.term.wx).as_bytes())?;
        for _i in status.len()..self.term.wx {
            self.term.write(b" ")?;
        }
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
