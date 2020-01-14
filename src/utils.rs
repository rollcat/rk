pub trait StringUtils {
    fn uslice(&self, start: usize, end: usize) -> String;
}

impl StringUtils for str {
    fn uslice(&self, start: usize, end: usize) -> String {
        self.chars().take(end).skip(start).collect()
    }
}
