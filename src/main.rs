extern crate termios;

use std::env;
use std::io;
use std::os::unix::io::AsRawFd;

use termios::*;

#[derive(Debug)]
struct Editor {
    cx: i16,
    cy: i16,

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
}

enum Command {
    Nothing,
    KeyPress { ch: u8 },
    Exit,
}

#[derive(Debug)]
struct Terminal {
    stdin: Box<io::Stdin>,
    orig_termios: Option<Termios>,
}

impl Terminal {
    fn new(stdin: io::Stdin) -> Terminal {
        Terminal {
            stdin: Box::new(stdin),
            orig_termios: Option::None,
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
}

fn read_char(reader: &mut io::Read) -> io::Result<u8> {
    let mut buffer = [0; 1];
    reader.read(&mut buffer)?;
    Ok(buffer[0])
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let t = Terminal::new(io::stdin());
    let mut e = Editor::new(t);

    println!("args: {:?}", args);
    println!("e: {:?}", e);

    e.term.enable_raw_mode().expect("could not enable raw mode");
    loop {
        let cmd = e.process_input().expect("process_input");
        match cmd {
            Command::Nothing => (),
            Command::Exit => break,
            Command::KeyPress { ch } => print!("ch: {:?}\r\n", ch),
        }
    }

    e.term
        .disable_raw_mode()
        .expect("could not disable raw mode");
    println!("e: {:?}", e);
}
