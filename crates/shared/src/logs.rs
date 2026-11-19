use std::range::Range;

#[derive(Default)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct Logs {
    text: String,
    ranges: Vec<Range<usize>>,
    len_new: usize,
    index: usize,
}

impl Logs {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            ranges: Vec::new(),
            len_new: 0,
            index: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub const fn len(&self) -> usize {
        self.ranges.len()
    }

    pub const fn len_new(&self) -> usize {
        self.len_new
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn push_err(&mut self, error: impl std::error::Error) {
        use std::fmt::Write;

        let start = self.text.len();
        let _ = writeln!(self.text, "{error}");

        let mut source = error.source();
        while let Some(cause) = source {
            let _ = writeln!(self.text, "Caused by: {cause}");
            source = cause.source();
        }

        self.add_log(start);
    }

    pub fn push_str(&mut self, s: &str) {
        let start = self.text.len();
        self.text.push_str(s);
        self.add_log(start);
    }

    pub fn push_fmt(&mut self, s: impl std::fmt::Display) {
        use std::fmt::Write;

        let start = self.text.len();
        let _ = write!(self.text, "{s}");
        self.add_log(start);
    }

    pub fn reset_new(&mut self) {
        self.len_new = 0;
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.ranges.iter().copied().map(|range| &self.text[range])
    }

    pub fn current(&self) -> Option<&str> {
        self.ranges
            .get(self.index)
            .copied()
            .map(|range| &self.text[range])
    }

    pub const fn next(&mut self) {
        self.index += 1;

        if self.index >= self.ranges.len() {
            self.index = 0;
        }
    }

    pub const fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.ranges.len().saturating_sub(1);
        } else {
            self.index -= 1;
        }
    }

    pub const fn first(&mut self) {
        self.index = 0;
    }

    pub const fn last(&mut self) {
        self.index = self.ranges.len().saturating_sub(1);
    }

    pub fn remove_current(&mut self) {
        if self.ranges.is_empty() {
            return;
        }

        self.ranges.remove(self.index);

        if self.ranges.is_empty() {
            self.clear();
            return;
        }

        if self.index >= self.ranges.len() {
            self.index = self.ranges.len().saturating_sub(1);
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.ranges.clear();
        self.index = 0;
    }

    fn add_log(&mut self, start: usize) {
        self.ranges.push(Range {
            start,
            end: self.text.len(),
        });
        self.len_new += 1;
    }
}
