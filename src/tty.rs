use std::fmt;
use std::io::{self, Stdin, Stdout, Write};

use termion::event::Event;
use termion::input::{MouseTerminal, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};

pub struct Terminal {
    pub wx: usize,
    pub wy: usize,
    events: termion::input::Events<Stdin>,
    term: MouseTerminal<RawTerminal<Stdout>>,
}

impl Terminal {
    pub fn new(stdin: Stdin, stdout: Stdout) -> io::Result<Terminal> {
        Ok(Terminal {
            wx: 0,
            wy: 0,
            events: stdin.events(),
            term: MouseTerminal::from(stdout.into_raw_mode()?),
        })
    }

    pub fn update(&mut self) -> io::Result<()> {
        let (x, y) = termion::terminal_size()?;
        self.wx = x as usize;
        self.wy = y as usize;
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        write!(self.term, "{}", termion::clear::All)?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> io::Result<()> {
        write!(self.term, "{}", termion::clear::CurrentLine)?;
        Ok(())
    }

    pub fn move_cursor(&mut self, x: usize, y: usize) -> io::Result<()> {
        write!(
            self.term,
            "{}",
            termion::cursor::Goto(x as u16 + 1, y as u16 + 1)
        )?;
        Ok(())
    }

    pub fn move_cursor_topleft(&mut self) -> io::Result<()> {
        write!(self.term, "{}", termion::cursor::Goto(1, 1))?;
        Ok(())
    }

    pub fn get_event(&mut self) -> io::Result<Event> {
        self.events.next().unwrap()
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        write!(self.term, "{}", termion::cursor::Hide)?;
        Ok(())
    }

    pub fn show_cursor(&mut self) -> io::Result<()> {
        write!(self.term, "{}", termion::cursor::Show)?;
        Ok(())
    }
}

impl Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.term.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.term.flush()
    }
}

impl fmt::Debug for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Terminal {{ wx: {wx}, wy: {wy} }}",
            wx = self.wx,
            wy = self.wy
        )
    }
}
