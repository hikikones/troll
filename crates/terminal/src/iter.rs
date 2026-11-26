use crate::{CSI, SGR_END};

#[derive(Debug)]
pub struct SgrIter<'a> {
    text: &'a str,
    index: usize,
    text_start: usize,
}

impl<'a> SgrIter<'a> {
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            index: 0,
            text_start: 0,
        }
    }

    pub fn only_sgr(self) -> impl Iterator<Item = &'a str> {
        self.with_filter(true)
    }

    pub fn only_text(self) -> impl Iterator<Item = &'a str> {
        self.with_filter(false)
    }

    fn with_filter(self, sgr: bool) -> impl Iterator<Item = &'a str> {
        self.filter(move |(_, is_sgr)| *is_sgr == sgr)
            .map(|(s, _)| s)
    }
}

impl<'a> Iterator for SgrIter<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let Some(sgr_start) = self.text[self.index..].find(CSI) else {
            self.index = self.text.len();

            if self.text_start == self.text.len() {
                return None;
            }

            let text = &self.text[self.text_start..];
            self.text_start = self.text.len();
            return Some((text, false));
        };

        let sgr_start = self.index + sgr_start;

        if self.text_start < sgr_start {
            let text = &self.text[self.text_start..sgr_start];
            self.index = sgr_start;
            self.text_start = sgr_start;
            return Some((text, false));
        }

        let Some(sgr_end) = self.text[(sgr_start + CSI.len())..].find(SGR_END) else {
            let text = &self.text[self.text_start..];
            self.index = self.text.len();
            self.text_start = self.text.len();
            return Some((text, false));
        };

        let sgr_end = self.index + sgr_end + CSI.len() + SGR_END.len();

        self.index = sgr_end;
        self.text_start = sgr_end;

        let sgr = &self.text[sgr_start..sgr_end];
        Some((sgr, true))
    }
}
