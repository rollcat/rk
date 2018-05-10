extern crate libc;
extern crate regex;
extern crate termios;

use std::env;
use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

use termios::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Editor {
    cx: u16,
    cy: u16,

    term: Terminal,
}

impl Editor {
    fn new(term: Terminal) -> Editor {
        Editor {
            cx: 0,
            cy: 0,
            term: term,
        }
    }

    fn refresh_screen(&mut self) -> io::Result<()> {
        let status = format!("? ~~  rk v{}  ~~", VERSION);
        self.term.hide_cursor()?;
        self.term.move_cursor_topleft()?;
        for _y in 1..self.term.wy {
            self.term.clear_line()?;
            let line = "~";
            self.term.write(line.as_bytes())?;
            self.term.write(b"\r\n")?;
        }
        self.term.write(status.as_bytes())?;
        self.term.move_cursor(self.cx, self.cy)?;
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
    stdout: Box<io::Write>,
    orig_termios: Option<Termios>,
    wx: u16,
    wy: u16,
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
        let ch0 = read_char(&mut self.stdin)?;
        Ok(if ch0 == '\x1b' {
            let ch1 = read_char(&mut self.stdin)?;
            if ch1 == '[' {
                let ch2 = read_char(&mut self.stdin)?;
                eprintln!("ch: {:?} {:?} {:?}", ch0, ch1, ch2);
                match ch2 {
                    'A' => KeyMod {
                        key: Key::Direction(Direction::Up),
                        ctrl: false,
                        meta: false,
                    },
                    'B' => KeyMod {
                        key: Key::Direction(Direction::Down),
                        ctrl: false,
                        meta: false,
                    },
                    'C' => KeyMod {
                        key: Key::Direction(Direction::Right),
                        ctrl: false,
                        meta: false,
                    },
                    'D' => KeyMod {
                        key: Key::Direction(Direction::Left),
                        ctrl: false,
                        meta: false,
                    },
                    _ => KeyMod {
                        key: Key::None,
                        ctrl: false,
                        meta: false,
                    },
                }
            } else {
                KeyMod {
                    key: Key::Char(ch0),
                    ctrl: false,
                    meta: false,
                }
            }
        } else {
            eprintln!("ch: {:?}", ch0);
            if ch0 == '\0' {
                KeyMod {
                    key: Key::None,
                    ctrl: false,
                    meta: false,
                }
            } else if (ch0 as u8) < 0x20 {
                KeyMod {
                    key: Key::Char((ch0 as u8 + 0x60) as char),
                    ctrl: true,
                    meta: false,
                }
            } else {
                KeyMod {
                    key: Key::Char(ch0),
                    ctrl: false,
                    meta: false,
                }
            }
        })
    }

    fn move_cursor_topleft(&mut self) -> io::Result<()> {
        self.write(b"\x1b[H")?;
        Ok(())
    }

    fn move_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
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
                    self.wx = ws.ws_col;
                    self.wy = ws.ws_row;
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
                self.wx = ws.ws_col;
                self.wy = ws.ws_row;
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

#[derive(Debug)]
enum Key {
    None,
    Char(char),
    Direction(Direction),
}

struct KeyMod {
    key: Key,
    ctrl: bool,
    meta: bool,
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
        write!(f, "{:?}>", self.key)
    }
}

fn read_char(reader: &mut io::Read) -> io::Result<char> {
    let mut buffer = [0; 1];
    reader.read(&mut buffer)?;
    Ok(buffer[0] as char)
}

fn main() {
    let _args: Vec<String> = env::args().collect();
    let t = Terminal::new(io::stdin(), io::stdout());
    let mut e = Editor::new(t);

    e.term.enable_raw_mode().expect("could not enable raw mode");
    e.term.update_window_size().expect("update_window_size");
    e.term.clear_screen().expect("clear_screen");
    e.refresh_screen().expect("refresh_screen");
    loop {
        e.term.update_window_size().expect("update_window_size");

        let KeyMod { key, ctrl, meta } = e.term.get_key().expect("get_key");
        let cmd = match key {
            Key::None => Command::Nothing,
            Key::Char('q') => Command::Exit,
            Key::Direction(d) => Command::Move(d),
            Key::Char(ch) => match ch {
                'h' => Command::Move(Direction::Left),
                'l' => Command::Move(Direction::Right),
                'k' => Command::Move(Direction::Up),
                'j' => Command::Move(Direction::Down),
                _ => Command::InsertCharacter(ch),
            },
        };

        match cmd {
            Command::Nothing => continue,
            Command::Exit => {
                e.refresh_screen().expect("refresh_screen");
                break;
            }
            Command::InsertCharacter(_) => (),
            Command::Move(d) => {
                match d {
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
                        if e.cy < e.term.wy - 1 {
                            e.cy += 1
                        }
                    } // ':' => {
                      //     e.cy = e.term.wy;
                      //     e.cx = 2;
                      // }
                }
                e.refresh_screen().expect("refresh_screen");
            }
        }
    }

    e.term
        .disable_raw_mode()
        .expect("could not disable raw mode");
    eprintln!("e: {:?}", e);
}
