use std::fmt;

#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub enum Key {
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

pub struct KeyMod {
    pub key: Key,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

impl KeyMod {
    pub fn new_none() -> KeyMod {
        KeyMod {
            key: Key::None,
            ctrl: false,
            meta: false,
            shift: false,
        }
    }
    pub fn new(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: false,
            meta: false,
            shift: false,
        }
    }
    pub fn new_ctrl(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: true,
            meta: false,
            shift: false,
        }
    }
    pub fn new_meta(key: Key) -> KeyMod {
        KeyMod {
            key: key,
            ctrl: false,
            meta: true,
            shift: false,
        }
    }
    pub fn new_shift(key: Key) -> KeyMod {
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
