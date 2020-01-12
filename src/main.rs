extern crate libc;
extern crate regex;
extern crate termios;

use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use termios::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Editor {
    cx: u32,
    cy: u32,
    oy: u32,

    term: Terminal,
    lines: Vec<String>,
}

impl Editor {
    fn new(term: Terminal) -> Editor {
        Editor {
            cx: 0,
            cy: 0,
            oy: 0,
            term: term,
            lines: Vec::new(),
        }
    }

    fn open(&mut self, fname: &Path) -> io::Result<()> {
        let file = io::BufReader::new(File::open(fname)?);
        for line in file.lines().map(|l| l.unwrap()) {
            self.lines.push(line);
        }
        Ok(())
    }

    fn refresh_screen(&mut self) -> io::Result<()> {
        if self.cy < self.oy {
            self.oy = self.cy;
        }
        if self.cy >= self.oy + self.term.wy {
            self.oy = self.cy - self.term.wy + 1;
        }

        let _status = format!("? ~~  rk v{}  ~~", VERSION);
        self.term.hide_cursor()?;
        self.term.move_cursor_topleft()?;
        for y in 0..(self.term.wy - 1) {
            let filerow = (y as u32 + self.oy) as usize;
            self.term.clear_line()?;
            if filerow < self.lines.len() {
                let line = &self.lines[filerow];
                if line.len() > self.term.wx as usize {
                    let line = &line[..(self.term.wx as usize) - 1];
                    self.term.write(line.as_bytes())?;
                    self.term.write(b"\\")?;
                } else {
                    self.term.write(line.as_bytes())?;
                }
            } else {
                self.term.write(b"~")?;
            }
            self.term.write(b"\r\n")?;
        }
        self.term.write("~".as_bytes())?;
        self.term.move_cursor(self.cx, self.cy - self.oy)?;
        self.term.show_cursor()?;
        self.term.flush()?;
        Ok(())
    }
}

#[derive(Debug)]
enum Command {
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

enum DeviceStatusQuery {
    WindowSize,
}

enum DeviceStatusResponse {
    WindowSize(libc::winsize),
}

struct Terminal {
    stdin: Box<io::Stdin>,
    stdout: Box<dyn io::Write>,
    orig_termios: Option<Termios>,
    wx: u32,
    wy: u32,
}

impl Terminal {
    fn new(stdin: io::Stdin, stdout: io::Stdout) -> Terminal {
        Terminal {
            stdin: Box::new(stdin),
            stdout: Box::new(io::BufWriter::new(stdout)),
            orig_termios: Option::None,
            wx: 0,
            wy: 0,
        }
    }

    fn enable_raw_mode(&mut self) -> io::Result<()> {
        let fd = self.stdin.as_raw_fd();
        let mut termios = Termios::from_fd(fd)?;
        tcgetattr(fd, &mut termios)?;
        self.orig_termios = Some(termios);
        termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        termios.c_oflag &= !(OPOST);
        termios.c_cflag |= CS8;
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 10;
        tcsetattr(fd, TCSAFLUSH, &termios)?;
        Ok(())
    }

    fn disable_raw_mode(&self) -> io::Result<()> {
        let fd = self.stdin.as_raw_fd();
        match self.orig_termios {
            Some(termios) => tcsetattr(fd, TCSAFLUSH, &termios)?,
            None => (),
        }
        Ok(())
    }

    fn get_key(&mut self) -> io::Result<KeyMod> {
        let mut buf = ['\0'; 4];
        buf[0] = read_char(&mut self.stdin)?;
        if buf[0] == '\x1b' {
            buf[1] = read_char(&mut self.stdin)?;
        }
        if (buf[0], buf[1]) == ('\x1b', '[') {
            buf[2] = read_char(&mut self.stdin)?;
        }

        Ok(match (buf[0], buf[1], buf[2]) {
            ('\x1b', '[', 'A') => KeyMod::new(Key::Direction(Up)),
            ('\x1b', '[', 'B') => KeyMod::new(Key::Direction(Down)),
            ('\x1b', '[', 'C') => KeyMod::new(Key::Direction(Right)),
            ('\x1b', '[', 'D') => KeyMod::new(Key::Direction(Left)),
            ('\x1b', '[', 'H') => KeyMod::new(Key::Home),
            ('\x1b', '[', 'F') => KeyMod::new(Key::End),
            ('\x1b', 'O', 'H') => KeyMod::new(Key::Home),
            ('\x1b', 'O', 'F') => KeyMod::new(Key::End),
            ('\x1b', '[', 'a') => KeyMod::new_shift(Key::Direction(Up)),
            ('\x1b', '[', 'b') => KeyMod::new_shift(Key::Direction(Down)),
            ('\x1b', '[', 'c') => KeyMod::new_shift(Key::Direction(Right)),
            ('\x1b', '[', 'd') => KeyMod::new_shift(Key::Direction(Left)),
            ('\x1b', '[', _) => {
                buf[3] = read_char(&mut self.stdin)?;
                match (buf[2], buf[3]) {
                    ('1', '~') => KeyMod::new(Key::Home),
                    ('3', '~') => KeyMod::new(Key::Delete),
                    ('4', '~') => KeyMod::new(Key::End),
                    ('5', '~') => KeyMod::new(Key::PageUp),
                    ('6', '~') => KeyMod::new(Key::PageDown),
                    ('7', '~') => KeyMod::new(Key::Home),
                    ('8', '~') => KeyMod::new(Key::End),
                    ('5', '^') => KeyMod::new_ctrl(Key::PageUp),
                    ('6', '^') => KeyMod::new_ctrl(Key::PageDown),
                    ('7', '^') => KeyMod::new_ctrl(Key::Home),
                    ('8', '^') => KeyMod::new_ctrl(Key::End),
                    _ => {
                        eprintln!("buf: {:?}", buf);
                        KeyMod::new_none()
                    }
                }
            }
            ('\x1b', ch, '\0') => KeyMod::new_meta(Key::Char(ch)),
            ('\0', '\0', '\0') => KeyMod::new_none(),
            ('\x7f', '\0', '\0') => KeyMod::new(Key::Backspace),
            (ch, '\0', '\0') => if (ch as u8) < 0x20 {
                KeyMod::new_ctrl(Key::Char((ch as u8 + 0x60) as char))
            } else {
                KeyMod::new(Key::Char(ch))
            },
            _ => {
                eprintln!("buf: {:?}", buf);
                KeyMod::new_none()
            }
        })
    }

    fn move_cursor_topleft(&mut self) -> io::Result<()> {
        self.write(b"\x1b[H")?;
        Ok(())
    }

    fn move_cursor(&mut self, x: u32, y: u32) -> io::Result<()> {
        self.write(format!("\x1b[{};{}H", y + 1, x + 1).as_bytes())?;
        Ok(())
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        self.write(b"\x1b[2J")?;
        Ok(())
    }

    fn clear_line(&mut self) -> io::Result<()> {
        self.write(b"\x1b[K")?;
        Ok(())
    }

    fn move_cursor_offscreen(&mut self) -> io::Result<()> {
        self.write(b"\x1b[999C\x1b[999B")?;
        Ok(())
    }

    fn device_status_report(&mut self, q: DeviceStatusQuery) -> io::Result<DeviceStatusResponse> {
        match q {
            DeviceStatusQuery::WindowSize => {
                self.write(b"\x1b[6n")?;
                self.flush()?;

                let mut out: [u8; 32] = [0; 32];
                self.stdin.read(&mut out)?;
                // TODO: this is too simple for regex but I'm too lazy
                let re = regex::Regex::new(
                    r"(?x)
                    \x1b\[
                    (?P<r>\d{1,4});
                    (?P<c>\d+)R",
                ).unwrap();
                let e = io::Error::new(io::ErrorKind::Other, "can't parse response");
                match std::str::from_utf8(&out) {
                    Ok(s) => match re.captures(s) {
                        Some(m) => Ok(DeviceStatusResponse::WindowSize(libc::winsize {
                            ws_col: m["c"].parse::<u16>().unwrap(),
                            ws_row: m["r"].parse::<u16>().unwrap(),
                            ws_xpixel: 0,
                            ws_ypixel: 0,
                        })),
                        None => Err(e),
                    },
                    Err(_) => Err(e),
                }
            }
        }
    }

    fn update_window_size_dsr(&mut self) -> io::Result<()> {
        // move cursor to bottom right corner and read position
        self.move_cursor_offscreen()?;
        match self.device_status_report(DeviceStatusQuery::WindowSize) {
            Ok(ds) => match ds {
                DeviceStatusResponse::WindowSize(ws) => {
                    self.wx = ws.ws_col as u32;
                    self.wy = ws.ws_row as u32;
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    fn update_window_size_ioctl(&mut self) -> io::Result<()> {
        let fd = self.stdin.as_raw_fd();
        let mut ws = libc::winsize {
            ws_col: 0,
            ws_row: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        match unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) } {
            -1 => Err(io::Error::last_os_error()),
            _ => {
                self.wx = ws.ws_col as u32;
                self.wy = ws.ws_row as u32;
                Ok(())
            }
        }
    }

    fn update_window_size(&mut self) -> io::Result<()> {
        match self.update_window_size_ioctl() {
            Ok(_) => Ok(()),
            Err(_) => self.update_window_size_dsr(),
        }
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.write(b"\x1b[?25l")?;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.write(b"\x1b[?25h")?;
        Ok(())
    }
}

impl Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl fmt::Debug for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Terminal {{ orig_termios: {:?}, wx: {}, wy: {} }}",
            self.orig_termios, self.wx, self.wy
        )
    }
}

#[derive(Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

use Direction::*;

#[derive(Debug)]
enum Key {
    None,
    Char(char),
    Direction(Direction),
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Delete,
}

struct KeyMod {
    key: Key,
    ctrl: bool,
    meta: bool,
    shift: bool,
}

impl KeyMod {
    fn new_none() -> KeyMod {
        KeyMod {
            key: Key::None,
            ctrl: false,
            meta: false,
            shift: false,
        }
    }
    fn new(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: false,
            meta: false,
            shift: false,
        }
    }
    fn new_ctrl(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: true,
            meta: false,
            shift: false,
        }
    }
    fn new_meta(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: false,
            meta: true,
            shift: false,
        }
    }
    fn new_shift(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: false,
            meta: false,
            shift: true,
        }
    }
}

impl fmt::Debug for KeyMod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "KeyMod <")?;
        if self.ctrl {
            write!(f, "C-")?;
        }
        if self.meta {
            write!(f, "M-")?;
        }
        if self.shift {
            write!(f, "S-")?;
        }
        write!(f, "{:?}>", self.key)
    }
}

fn read_char(reader: &mut dyn io::Read) -> io::Result<char> {
    let mut buffer = [0; 1];
    reader.read(&mut buffer)?;
    Ok(buffer[0] as char)
}

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
            ctrl: _,
            meta: _,
            shift: _,
        } = km;
        let cmd = match key {
            Key::None => Command::Nothing,
            Key::Char('q') => Command::Exit,
            Key::Direction(d) => Command::Move(d),
            Key::Char(ch) => match ch {
                'h' => Command::Move(Left),
                'l' => Command::Move(Right),
                'k' => Command::Move(Up),
                'j' => Command::Move(Down),
                _ => Command::InsertCharacter(ch),
            },
            Key::PageUp => Command::MovePageUp,
            Key::PageDown => Command::MovePageDown,
            Key::Home => Command::MoveLineHome,
            Key::End => Command::MoveLineEnd,
            Key::Backspace => Command::Erase(Left),
            Key::Delete => Command::Erase(Right),
        };

        match cmd {
            Command::Nothing => continue,
            Command::Exit => {
                e.refresh_screen().expect("refresh_screen");
                break;
            }
            Command::InsertCharacter(_) => (),
            Command::Move(d) => match d {
                Direction::Left => {
                    if e.cx > 0 {
                        e.cx -= 1
                    }
                }
                Direction::Right => {
                    if e.cx < e.term.wx - 1 {
                        e.cx += 1
                    }
                }
                Direction::Up => {
                    if e.cy > 0 {
                        e.cy -= 1
                    }
                }
                Direction::Down => {
                    if e.cy < (e.lines.len() - 1) as u32 {
                        e.cy += 1
                    }
                }
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
            Command::Erase(Right) => {
                if e.cx < e.term.wx - 1 {
                    e.cx += 1
                }
            }
            Command::Erase(_) => (),
        }
        e.refresh_screen().expect("refresh_screen");
    }

    e.term
        .disable_raw_mode()
        .expect("could not disable raw mode");
    eprintln!("e: {:?}", e);
}
