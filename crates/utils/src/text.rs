use unicode_segmentation::{GraphemeIndices, Graphemes, UnicodeSegmentation};

pub fn char_width(c: char) -> u16 {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) as u16
}

pub fn str_width(s: &str) -> u16 {
    unicode_width::UnicodeWidthStr::width(s) as u16
}

pub fn display_width(s: &str) -> u16 {
    textwrap::core::display_width(s) as u16
}

pub fn text_wrap(s: &mut String, max_width: u16) {
    textwrap::fill_inplace(s, max_width as usize);
}

pub fn graphemes(s: &str) -> Graphemes<'_> {
    s.graphemes(true)
}

pub fn grapheme_indices(s: &str) -> GraphemeIndices<'_> {
    s.grapheme_indices(true)
}

#[derive(Debug)]
pub struct GraphemeAnsiIter<'a> {
    text: &'a str,
    graphemes: GraphemeIndices<'a>,
}

#[derive(Debug)]
pub enum GraphemeOrAnsi<'a> {
    Grapheme(&'a str),
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
        let Some((start, g)) = self.graphemes.next() else {
            return None;
        };

        if g == "\x1b" {
            match self.graphemes.find(|(_, g)| *g == "m") {
                Some((end, _)) => Some(GraphemeOrAnsi::Ansi(&self.text[start..end + 1])),
                None => None,
            }
        } else {
            Some(GraphemeOrAnsi::Grapheme(g))
        }
    }
}
