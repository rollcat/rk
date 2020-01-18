use std::cmp::{max, min};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crossterm::{
    event::{Event, KeyCode, KeyEvent, MouseEvent},
    style,
    terminal::{Clear, ClearType},
    QueueableCommand,
};

use keys;
use tty;
use utils::*;

use std::boxed::Box;
use std::collections::HashMap;
use std::error::Error;
use std::result::Result;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Exit;

#[derive(Debug, Clone)]
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

    // Key bindings
    keys: HashMap<KeyEvent, Command>,
}

#[derive(Debug, Clone)]
pub enum Command {
    Nothing,
    InsertCharacter(char),
    Move(Direction),
    MoveTo(usize, usize),
    MovePageUp,
    MovePageDown,
    MoveLineHome,
    MoveLineEnd,
    Erase(Direction),
    Panic(String),
    Exit,
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
            keys: Editor::newkeys(),
        }
    }

    fn newkeys() -> HashMap<KeyEvent, Command> {
        let mut keys = HashMap::new();
        keys.insert(keys::must_parse("c-q"), Command::Exit);
        keys.insert(
            keys::must_parse("m-q"),
            Command::Panic("forced panic".into()),
        );
        keys.insert(keys::must_parse("up"), Command::Move(Direction::Up));
        keys.insert(keys::must_parse("down"), Command::Move(Direction::Down));
        keys.insert(keys::must_parse("left"), Command::Move(Direction::Left));
        keys.insert(
            keys::must_parse("right"),
            Command::Move(Direction::Right),
        );
        keys.insert(keys::must_parse("a-b"), Command::Move(Direction::Left));
        keys.insert(keys::must_parse("a-f"), Command::Move(Direction::Right));
        keys.insert(keys::must_parse("c-m"), Command::InsertCharacter('\n'));
        keys.insert(keys::must_parse("enter"), Command::InsertCharacter('\n'));
        keys.insert(keys::must_parse("pageup"), Command::MovePageUp);
        keys.insert(keys::must_parse("pagedown"), Command::MovePageDown);
        keys.insert(keys::must_parse("home"), Command::MoveLineHome);
        keys.insert(keys::must_parse("end"), Command::MoveLineEnd);
        keys.insert(
            keys::must_parse("backspace"),
            Command::Erase(Direction::Left),
        );
        keys.insert(
            keys::must_parse("delete"),
            Command::Erase(Direction::Right),
        );
        keys
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.term.init()?;
        self.update_screen()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<Option<Exit>, Box<dyn Error>> {
        let cmd = self.update_input()?;
        let status = self.exec_cmd(cmd)?;
        self.scroll_to_cursor();
        self.update_screen()?;
        Ok(status)
    }

    pub fn deinit(&mut self) -> Result<(), Box<dyn Error>> {
        self.term.deinit()?;
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

    fn update_input(&mut self) -> Result<Command, Box<dyn Error>> {
        let ev = self.term.get_event()?;
        self.message = String::from(format!("rk v{} ev{:?}", VERSION, ev));
        Ok(match ev {
            None => Command::Nothing,
            Some(Event::Key(k)) => {
                if let Some(cmd) = self.keys.get(&k) {
                    cmd.clone()
                } else {
                    if k.modifiers.is_empty() {
                        if let KeyCode::Char(c) = k.code {
                            return Ok(Command::InsertCharacter(c));
                        }
                    }
                    self.message =
                        format!("key not bound: {}", keys::display(k));
                    Command::Nothing
                }
            }
            Some(Event::Resize(_, _)) => Command::Nothing,
            Some(Event::Mouse(m)) => match m {
                MouseEvent::Down(_, x, y, _) => {
                    Command::MoveTo(x as usize, y as usize)
                }
                MouseEvent::Up(_, x, y, _) => {
                    Command::MoveTo(x as usize, y as usize)
                }
                MouseEvent::Drag(_, x, y, _) => {
                    Command::MoveTo(x as usize, y as usize)
                }
                MouseEvent::ScrollUp(_, _, _) => Command::MovePageUp,
                MouseEvent::ScrollDown(_, _, _) => Command::MovePageDown,
            },
        })
    }

    fn exec_cmd(
        &mut self,
        cmd: Command,
    ) -> Result<Option<Exit>, Box<dyn Error>> {
        match cmd {
            Command::Nothing => (),
            Command::Panic(s) => panic!(s),
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
            Command::MoveTo(x, y) => {
                self.exec_cmd_move_to(x, y);
            }
            Command::MovePageUp => {
                self.cx = 0;
                self.cy = max(0, self.cy - self.term.wy);
            }
            Command::MovePageDown => {
                self.cx = 0;
                self.cy = min(self.lines.len() - 1, self.cy + self.term.wy);
            }
            Command::MoveLineHome => {
                self.cx = 0;
            }
            Command::MoveLineEnd => {
                self.cx = self.lines[self.cy].ulen() - 1;
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
        self.cy = min(self.cy, self.lines.len() - 1);
        self.cx = min(self.cx, self.lines[self.cy].ulen());
    }

    fn exec_cmd_move_to(&mut self, x: usize, y: usize) {
        self.cy = max(0, min(y, self.lines.len() - 1));
        self.cx = max(0, min(x, self.lines[self.cy].ulen()));
    }

    fn exec_cmd_insert(&mut self, ch: char) {
        match ch {
            '\n' => {
                // Take the current line, and break it in two
                let line = self.lines[self.cy].clone();
                self.lines[self.cy] = line.uslice(0, self.cx);
                self.cy += 1;
                self.lines
                    .insert(self.cy, line.uslice(self.cx, line.ulen()));
                self.cx = 0;
            }
            _ => {
                // Insert in the middle of the current line
                let line = &self.lines[self.cy];
                let len = line.ulen();
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
                    self.cx = self.lines[self.cy].ulen();
                    let mut newline =
                        String::with_capacity(self.cx + line.ulen());
                    newline.push_str(&self.lines[self.cy]);
                    newline.push_str(&line);
                    self.lines[self.cy] = newline;
                } else {
                    // remove from the middle
                    let line = &self.lines[self.cy];
                    let len = line.ulen();
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

    fn update_screen(&mut self) -> Result<(), Box<dyn Error>> {
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
        if self.term.wy == 0 || self.term.wx == 0 {
            return Ok(());
        }

        let status = format!(
            "? {fname} {line}:{col} -- {message}",
            fname = self.fname,
            col = self.cx,
            line = self.cy + 1,
            message = self.message,
        );
        self.term
            .stdout
            .queue(crossterm::cursor::Hide)?
            .queue(crossterm::cursor::MoveTo(0, 0))?;
        for y in self.oy..min(self.oy + self.term.wy, self.lines.len()) {
            let line = &self.lines[y];
            let line = line.uslice(self.ox, self.ox + self.term.wx + 1);
            self.term
                .stdout
                .queue(Clear(ClearType::CurrentLine))?
                .queue(style::Print(line))?
                .queue(style::Print("\r\n"))?
                .queue(style::ResetColor)?;
        }
        self.term
            .stdout
            .queue(style::SetForegroundColor(style::Color::Blue))?;
        for _y in min(self.oy + self.term.wy, self.lines.len())
            ..(self.oy + self.term.wy)
        {
            self.term
                .stdout
                .queue(Clear(ClearType::CurrentLine))?
                .queue(style::Print("~\r\n"))?;
        }
        self.term
            .stdout
            .queue(style::SetBackgroundColor(style::Color::Blue))?
            .queue(style::SetForegroundColor(style::Color::Black))?
            .queue(style::Print(status.uslice(0, self.term.wx)))?;
        for _i in status.ulen()..(self.term.wx + 1) {
            self.term.stdout.queue(style::Print(" "))?;
        }
        self.term
            .stdout
            .queue(style::ResetColor)?
            .queue(crossterm::cursor::MoveTo(
                (self.cx - self.ox) as u16,
                (self.cy - self.oy) as u16,
            ))?
            .queue(crossterm::cursor::Show)?
            .flush()?;
        Ok(())
    }
}
