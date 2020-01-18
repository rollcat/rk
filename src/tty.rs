use std::boxed::Box;
use std::error::Error;
use std::fmt;
use std::io::{Stdout, Write};
use std::result::Result;
use std::time::Duration;

use crossterm::{
    event::{self, Event},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

pub struct Terminal {
    pub wx: usize,
    pub wy: usize,
    pub stdout: Stdout,
}

pub fn is_tty<T: std::os::unix::io::AsRawFd>(stream: &T) -> bool {
    let fd = stream.as_raw_fd();
    unsafe { libc::isatty(fd) == 1 }
}

impl Terminal {
    pub fn new(stdout: Stdout) -> Result<Terminal, Box<dyn Error>> {
        let (x, y) = crossterm::terminal::size()?;
        Ok(Terminal {
            wx: x as usize - 1,
            wy: y as usize - 1,
            stdout: stdout,
        })
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        crossterm::terminal::enable_raw_mode()?;
        self.stdout
            .queue(Clear(ClearType::All))?
            .queue(crossterm::event::EnableMouseCapture)?
            .flush()?;
        Ok(())
    }

    pub fn deinit(&mut self) -> Result<(), Box<dyn Error>> {
        self.stdout
            .queue(crossterm::style::ResetColor)?
            .queue(crossterm::event::DisableMouseCapture)?
            .queue(crossterm::style::ResetColor)?
            .queue(Clear(ClearType::All))?
            .queue(crossterm::cursor::MoveTo(0, 0))?
            .flush()?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn get_event(&mut self) -> Result<Option<Event>, Box<dyn Error>> {
        if event::poll(Duration::from_millis(1000))? {
            match event::read()? {
                Event::Resize(w, h) => {
                    self.wx = w as usize - 1;
                    self.wy = h as usize - 1;
                    Ok(Some(Event::Resize(w, h)))
                }
                ev => Ok(Some(ev)),
            }
        } else {
            Ok(None)
        }
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
