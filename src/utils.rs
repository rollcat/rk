pub trait StringUtils {
    fn uslice(&self, start: usize, end: usize) -> String;
    fn ulen(&self) -> usize;
}

impl StringUtils for str {
    fn uslice(&self, start: usize, end: usize) -> String {
        self.chars().take(end).skip(start).collect()
    }
    fn ulen(&self) -> usize {
        self.chars().count()
    }
}
