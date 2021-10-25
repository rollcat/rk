#[cfg(test)]
mod tests {
    use crate::utils::*;

    #[test]
    fn test_uslice_basic() {
        assert_eq!("".uslice(0, 0), "");
        assert_eq!("".uslice(1, 2), "");

        assert_eq!("a".uslice(0, 0), "");
        assert_eq!("a".uslice(0, 1), "a");
        assert_eq!("a".uslice(0, 2), "a");
        assert_eq!("a".uslice(1, 2), "");

        assert_eq!("ab".uslice(0, 1), "a");
        assert_eq!("ab".uslice(0, 2), "ab");
        assert_eq!("ab".uslice(0, 3), "ab");

        assert_eq!("ab".uslice(1, 1), "");
        assert_eq!("ab".uslice(2, 3), "");
        assert_eq!("ab".uslice(1, 2), "b");
        assert_eq!("ab".uslice(1, 3), "b");
    }

    #[test]
    fn test_uslice_unicode() {
        assert_eq!("καλημέρα".uslice(0, 0), "");
        assert_eq!("καλημέρα".uslice(0, 4), "καλη");
        assert_eq!("καλημέρα".uslice(4, 8), "μέρα");
        assert_eq!("καλημέρα".uslice(4, 6), "μέ");
        assert_eq!("καλημέρα".uslice(4, 8), "μέρα");
    }
}
