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
}

impl Editor {
    fn new() -> Editor {
        Editor{
            cx: 1,
            cy: 1,
        }
    }
}

fn enable_raw_mode(fd: RawFd) -> io::Result<()> {
    let mut termios = Termios::from_fd(fd)?;
    tcgetattr(fd, &mut termios)?;
    termios.c_lflag &= !(ECHO);
    tcsetattr(fd, TCSAFLUSH, &termios)?;
    Ok(())
}

fn disable_raw_mode(fd: RawFd) -> io::Result<()> {
    let mut termios = Termios::from_fd(fd)?;
    tcgetattr(fd, &mut termios)?;
    termios.c_lflag |= (ECHO);
    tcsetattr(fd, TCSAFLUSH, &termios)?;
    Ok(())
}

fn read_char(fd: &mut io::Read) -> io::Result<u8> {
    let mut buffer = [0; 1];
    fd.read(&mut buffer)?;
    Ok(buffer[0])
}

fn main() {
    let e = Editor::new();
    let args: Vec<String> = env::args().collect();
    let mut stdin = io::stdin();
    let fd = stdin.as_raw_fd();

    println!("args: {:?}", args);

    enable_raw_mode(fd).expect("could not enable raw mode");
    loop {
        let ch = read_char(&mut stdin).expect("could not read char");
        println!("ch: {:?}", ch);
        if ch == 'q' as u8 {
            break
        }
    }

    disable_raw_mode(fd).expect("could not disable raw mode");
}
