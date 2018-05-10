extern crate libc;
extern crate termios;

use std::env;
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;

use termios::*;

#[derive(Debug)]
struct Editor {
    cx: u16,
    cy: u16,

    term: Terminal,
}

impl Editor {
    fn new(term: Terminal) -> Editor {
        Editor {
            cx: 1,
            cy: 1,
            term: term,
        }
    }

    fn process_input(&mut self) -> io::Result<Command> {
        let ch = self.term.get_key()?;
        Ok(match ch {
            /* wait  */ 0x00 => Command::Nothing,
            /* C-q   */ 0x11 => Command::Exit,
            /* other */ _ => Command::KeyPress { ch: ch },
        })
    }

    fn refresh_screen(&mut self) -> io::Result<()> {
        self.term.stdout.write(b"\x1b[2J")?;
        self.term.stdout.write(b"\x1b[H")?;
        self.draw_rows()?;
        self.term.stdout.write(b"\x1b[H")?;
        Ok(())
    }

    fn draw_rows(&mut self) -> io::Result<()> {
        for _y in 1..self.term.wy {
            self.term.stdout.write(b"~\r\n")?;
        }
        Ok(())
    }
}

enum Command {
    Nothing,
    KeyPress { ch: u8 },
    Exit,
}

#[derive(Debug)]
struct Terminal {
    stdin: Box<io::Stdin>,
    stdout: Box<io::Stdout>,
    orig_termios: Option<Termios>,
    wx: u16,
    wy: u16,
}

impl Terminal {
    fn new(stdin: io::Stdin, stdout: io::Stdout) -> Terminal {
        Terminal {
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
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
        termios.c_cc[VTIME] = 1;
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

    fn get_key(&mut self) -> io::Result<u8> {
        read_char(&mut self.stdin)
    }

    fn update_window_size(&mut self) -> io::Result<()> {
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
}

fn read_char(reader: &mut io::Read) -> io::Result<u8> {
    let mut buffer = [0; 1];
    reader.read(&mut buffer)?;
    Ok(buffer[0])
}

fn main() {
    let _args: Vec<String> = env::args().collect();
    let t = Terminal::new(io::stdin(), io::stdout());
    let mut e = Editor::new(t);

    e.term.enable_raw_mode().expect("could not enable raw mode");
    e.refresh_screen().expect("refresh_screen");
    loop {
        e.term.update_window_size().expect("update_window_size");
        let cmd = e.process_input().expect("process_input");
        match cmd {
            Command::Nothing => (),
            Command::Exit => {
                e.refresh_screen().expect("refresh_screen");
                break;
            }
            Command::KeyPress { ch: _ } => {
                e.refresh_screen().expect("refresh_screen");
            }
        }
    }

    e.term
        .disable_raw_mode()
        .expect("could not disable raw mode");
    println!("e: {:?}", e);
}
