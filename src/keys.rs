use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn _parse_code(s: &str) -> Option<KeyCode> {
    match s.len() {
        0 => None,
        1 => Some(KeyCode::Char(s.chars().next().expect("s.len"))),
        _ => match s {
            "up" => Some(KeyCode::Up),
            "down" => Some(KeyCode::Down),
            "left" => Some(KeyCode::Left),
            "right" => Some(KeyCode::Right),
            "pageup" => Some(KeyCode::PageUp),
            "pagedown" => Some(KeyCode::PageDown),
            "home" => Some(KeyCode::Home),
            "end" => Some(KeyCode::End),
            "backspace" => Some(KeyCode::Backspace),
            "delete" => Some(KeyCode::Delete),
            "insert" => Some(KeyCode::Insert),
            "enter" => Some(KeyCode::Enter),
            "tab" => Some(KeyCode::Tab),
            "backtab" => Some(KeyCode::BackTab),
            "esc" => Some(KeyCode::Esc),
            _ => {
                if s.chars().next().expect("s.len") == 'f' {
                    match s[1..].parse::<u8>() {
                        Ok(i) => Some(KeyCode::F(i)),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        },
    }
}

fn _parse_mod(s: &str) -> Option<KeyModifiers> {
    match s.to_ascii_lowercase().chars().next() {
        Some(ch) => match ch {
            'c' => Some(KeyModifiers::CONTROL),
            's' => Some(KeyModifiers::SHIFT),
            'm' => Some(KeyModifiers::ALT),
            'a' => Some(KeyModifiers::ALT),
            _ => None,
        },
        None => None,
    }
}

pub fn parse(s: &str) -> Option<KeyEvent> {
    let ss: Vec<&str> = s.split("-").collect();
    let mut mods = KeyModifiers::empty();
    if ss.len() == 0 {
        return None;
    }
    if ss.len() >= 2 {
        for x in &ss[..ss.len() - 1] {
            if let Some(m) = _parse_mod(x) {
                mods.insert(m);
            }
        }
    }
    match _parse_code(ss.last().expect("ss.len")) {
        Some(c) => Some(KeyEvent {
            code: c,
            modifiers: mods,
        }),
        None => None,
    }
}

pub fn must_parse(s: &str) -> KeyEvent {
    match parse(s) {
        Some(k) => k,
        None => panic!("Cannot parse: {:?}", s),
    }
}

pub fn display(ke: KeyEvent) -> String {
    format!(
        "{mods}{key}",
        mods = {
            let mut out = String::with_capacity(6);
            if ke.modifiers.contains(KeyModifiers::CONTROL) {
                out.push_str("C-");
            }
            if ke.modifiers.contains(KeyModifiers::ALT) {
                out.push_str("A-");
            }
            if ke.modifiers.contains(KeyModifiers::SHIFT) {
                out.push_str("S-");
            }
            out
        },
        key = match ke.code {
            KeyCode::Up => "up".into(),
            KeyCode::Down => "down".into(),
            KeyCode::Left => "left".into(),
            KeyCode::Right => "right".into(),
            KeyCode::PageUp => "pageup".into(),
            KeyCode::PageDown => "pagedown".into(),
            KeyCode::Home => "home".into(),
            KeyCode::End => "end".into(),
            KeyCode::Backspace => "backspace".into(),
            KeyCode::Delete => "delete".into(),
            KeyCode::Insert => "insert".into(),
            KeyCode::Enter => "enter".into(),
            KeyCode::Tab => "tab".into(),
            KeyCode::BackTab => "backtab".into(),
            KeyCode::Esc => "esc".into(),
            KeyCode::Null => "null".into(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::F(i) => format!("f{}", i),
        },
    )
}
