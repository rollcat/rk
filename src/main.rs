extern crate termios;

use std::env;
use std::io;
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;

use termios::*;

#[derive(Debug)]
struct Editor {
    cx: i16,
    cy: i16,

    fd: RawFd,
    orig_termios: Option<Termios>,
}

impl Editor {
    fn new(fd: RawFd) -> Editor {
        Editor {
            cx: 1,
            cy: 1,
            fd: fd,
            orig_termios: Option::None,
        }
    }

    fn enable_raw_mode(&mut self) -> io::Result<()> {
        let mut termios = Termios::from_fd(self.fd)?;
        tcgetattr(self.fd, &mut termios)?;
        self.orig_termios = Some(termios);
        termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        termios.c_oflag &= !(OPOST);
        termios.c_cflag |= CS8;
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 1;
        tcsetattr(self.fd, TCSAFLUSH, &termios)?;
        Ok(())
    }

    fn disable_raw_mode(&self) -> io::Result<()> {
        match self.orig_termios {
            Some(termios) => tcsetattr(self.fd, TCSAFLUSH, &termios)?,
            None => (),
        }
        Ok(())
    }
}

fn read_char(fd: &mut io::Read) -> io::Result<u8> {
    let mut buffer = [0; 1];
    fd.read(&mut buffer)?;
    Ok(buffer[0])
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut stdin = io::stdin();
    let mut e = Editor::new(stdin.as_raw_fd());

    println!("args: {:?}", args);
    println!("e: {:?}", e);

    e.enable_raw_mode().expect("could not enable raw mode");
    loop {
        let ch = read_char(&mut stdin).expect("could not read char");
        if ch != 0 {
            print!("ch: {:?}\r\n", ch);
        }
        if ch == 'q' as u8 {
            break;
        }
    }

    e.disable_raw_mode().expect("could not disable raw mode");
    println!("e: {:?}", e);
}
