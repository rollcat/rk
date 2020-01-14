extern crate libc;
extern crate regex;
extern crate termios;

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

use termios::*;

use keys::Direction::*;
use keys::*;

enum DeviceStatusQuery {
    WindowSize,
}

enum DeviceStatusResponse {
    WindowSize(libc::winsize),
}

pub struct Terminal {
    stdin: Box<io::Stdin>,
    stdout: Box<dyn io::Write>,
    orig_termios: Option<Termios>,
    pub wx: usize,
    pub wy: usize,
}

fn read_char(reader: &mut dyn io::Read) -> io::Result<char> {
    let mut buffer = [0; 1];
    reader.read(&mut buffer)?;
    Ok(buffer[0] as char)
}

impl Terminal {
    pub fn new(stdin: io::Stdin, stdout: io::Stdout) -> Terminal {
        Terminal {
            stdin: Box::new(stdin),
            stdout: Box::new(io::BufWriter::new(stdout)),
            orig_termios: Option::None,
            wx: 0,
            wy: 0,
        }
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
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

    pub fn disable_raw_mode(&self) -> io::Result<()> {
        let fd = self.stdin.as_raw_fd();
        match self.orig_termios {
            Some(termios) => tcsetattr(fd, TCSAFLUSH, &termios)?,
            None => (),
        }
        Ok(())
    }

    pub fn get_key(&mut self) -> io::Result<KeyMod> {
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
                        // eprintln!("buf: {:?}", buf);
                        KeyMod::new_none()
                    }
                }
            }
            ('\x1b', ch, '\0') => KeyMod::new_meta(Key::Char(ch)),
            ('\0', '\0', '\0') => KeyMod::new_none(),
            ('\x7f', '\0', '\0') => KeyMod::new(Key::Backspace),
            (ch, '\0', '\0') => {
                if (ch as u8) < 0x20 {
                    KeyMod::new_ctrl(Key::Char((ch as u8 + 0x60) as char))
                } else {
                    KeyMod::new(Key::Char(ch))
                }
            }
            _ => {
                // eprintln!("buf: {:?}", buf);
                KeyMod::new_none()
            }
        })
    }

    pub fn move_cursor_topleft(&mut self) -> io::Result<()> {
        self.write(b"\x1b[H")?;
        Ok(())
    }

    pub fn move_cursor(&mut self, x: usize, y: usize) -> io::Result<()> {
        self.write(format!("\x1b[{};{}H", y + 1, x + 1).as_bytes())?;
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.write(b"\x1b[2J")?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> io::Result<()> {
        self.write(b"\x1b[K")?;
        Ok(())
    }

    fn move_cursor_offscreen(&mut self) -> io::Result<()> {
        self.write(b"\x1b[999C\x1b[999B")?;
        Ok(())
    }

    fn device_status_report(
        &mut self,
        q: DeviceStatusQuery,
    ) -> io::Result<DeviceStatusResponse> {
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
                )
                .unwrap();
                let e = io::Error::new(
                    io::ErrorKind::Other,
                    "can't parse response",
                );
                match std::str::from_utf8(&out) {
                    Ok(s) => match re.captures(s) {
                        Some(m) => Ok(DeviceStatusResponse::WindowSize(
                            libc::winsize {
                                ws_col: m["c"].parse::<u16>().unwrap(),
                                ws_row: m["r"].parse::<u16>().unwrap(),
                                ws_xpixel: 0,
                                ws_ypixel: 0,
                            },
                        )),
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
                    self.wx = ws.ws_col as usize;
                    self.wy = ws.ws_row as usize;
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
                self.wx = ws.ws_col as usize;
                self.wy = ws.ws_row as usize;
                Ok(())
            }
        }
    }

    pub fn update_window_size(&mut self) -> io::Result<()> {
        match self.update_window_size_ioctl() {
            Ok(_) => Ok(()),
            Err(_) => self.update_window_size_dsr(),
        }
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.write(b"\x1b[?25l")?;
        Ok(())
    }

    pub fn show_cursor(&mut self) -> io::Result<()> {
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
