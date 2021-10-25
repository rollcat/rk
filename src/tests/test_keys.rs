#[cfg(test)]
mod tests {
    use crate::keys::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_parse_nomod() {
        assert_eq!(
            parse("c"),
            Some(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::empty(),
            })
        );
        assert_eq!(
            parse("g"),
            Some(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::empty(),
            })
        );
        assert_eq!(
            parse("h"),
            Some(KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::empty(),
            })
        );
        assert_eq!(
            parse("a"),
            Some(KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::empty(),
            })
        );
    }

    #[test]
    fn test_parse_one_mod() {
        assert_eq!(
            parse("C-c"),
            Some(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            })
        );
        assert_eq!(
            parse("C-a"),
            Some(KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            })
        );
        assert_eq!(
            parse("c-a"),
            Some(KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            })
        );
        assert_eq!(
            parse("a-C"),
            Some(KeyEvent {
                code: KeyCode::Char('C'),
                modifiers: KeyModifiers::ALT,
            })
        );
    }

    #[test]
    fn test_parse_many_mods() {
        assert_eq!(
            parse("C-a-c"),
            Some(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
            })
        );
        assert_eq!(
            parse("c-a-q"),
            Some(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
            })
        );
    }

    #[test]
    fn test_parse_nonalnum() {
        assert_eq!(
            parse("enter"),
            Some(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::empty(),
            })
        );
        assert_eq!(
            parse("Backspace"),
            Some(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::empty(),
            })
        );
        assert_eq!(
            parse("c-f3"),
            Some(KeyEvent {
                code: KeyCode::F(3),
                modifiers: KeyModifiers::CONTROL,
            })
        );
    }
}
