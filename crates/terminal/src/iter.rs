use utils::{Grapheme, GraphemeIndices, grapheme_indices};

use crate::{CSI, ESC, SGR_END};

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

        let sgr_end = self.index + sgr_end + CSI.len();

        self.index = sgr_end + SGR_END.len();
        self.text_start = self.index;

        let sgr = &self.text[sgr_start..self.index];
        Some((sgr, true))
    }
}

#[derive(Debug)]
pub struct GraphemeAnsiIter<'a> {
    text: &'a str,
    graphemes: GraphemeIndices<'a>,
}

#[derive(Debug)]
pub enum GraphemeOrAnsi<'a> {
    Grapheme(Grapheme<'a>),
    Ansi(&'a str),
}

impl<'a> GraphemeAnsiIter<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            graphemes: grapheme_indices(text),
        }
    }
}

impl<'a> Iterator for GraphemeAnsiIter<'a> {
    type Item = GraphemeOrAnsi<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, g) = self.graphemes.next()?;

        if g.equals(ESC) {
            match self.graphemes.find(|(_, g)| g.equals(SGR_END)) {
                Some((end, _)) => Some(GraphemeOrAnsi::Ansi(&self.text[start..end + 1])),
                None => None,
            }
        } else {
            Some(GraphemeOrAnsi::Grapheme(g))
        }
    }
}
