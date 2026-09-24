use std::{fmt::Display, ops::Range};

pub trait StringExt {
    fn slice(&self, range: Range<usize>) -> &str;
    fn push_str_get_range(&mut self, s: &str) -> Range<usize>;
    fn push_str_get_range2(&mut self, s1: &str, s2: &str) -> (Range<usize>, Range<usize>);
    fn extend_get_range<'a>(&mut self, iter: impl IntoIterator<Item = &'a str>) -> Range<usize>;
    fn write(&mut self, content: impl Display);
    fn write_get_range(&mut self, content: impl Display) -> Range<usize>;
}

impl StringExt for String {
    fn slice(&self, range: Range<usize>) -> &str {
        &self[range]
    }

    fn push_str_get_range(&mut self, s: &str) -> Range<usize> {
        let start = self.len();
        self.push_str(s);
        start..self.len()
    }

    fn push_str_get_range2(&mut self, s1: &str, s2: &str) -> (Range<usize>, Range<usize>) {
        let start = self.len();
        self.push_str(s1);
        let middle = self.len();
        self.push_str(s2);
        (start..middle, middle..self.len())
    }

    fn extend_get_range<'a>(&mut self, iter: impl IntoIterator<Item = &'a str>) -> Range<usize> {
        let start = self.len();
        self.extend(iter);
        start..self.len()
    }

    fn write(&mut self, content: impl Display) {
        let _ = std::fmt::Write::write_fmt(self, format_args!("{content}"));
    }

    fn write_get_range(&mut self, content: impl Display) -> Range<usize> {
        let start = self.len();
        let _ = std::fmt::Write::write_fmt(self, format_args!("{content}"));
        start..self.len()
    }
}
