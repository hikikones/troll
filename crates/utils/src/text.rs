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

pub fn graphemes<'a>(s: &'a str) -> Graphemes<'a> {
    Graphemes::new(s)
}

pub fn grapheme_indices<'a>(s: &'a str) -> GraphemeIndices<'a> {
    GraphemeIndices::new(s)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Grapheme<'a>(pub &'a str);

impl<'a> Grapheme<'a> {
    pub fn is_newline(&self) -> bool {
        self.0.contains('\n')
    }

    pub fn is_whitespace(&self) -> bool {
        self.0.chars().all(char::is_whitespace)
    }

    pub fn equals(&self, other: &str) -> bool {
        self.0.eq(other)
    }

    pub fn not_equals(&self, other: &str) -> bool {
        self.0.ne(other)
    }

    pub fn compare(&self, other: &str) -> std::cmp::Ordering {
        self.0.cmp(other)
    }

    pub fn width(&self) -> usize {
        unicode_width::UnicodeWidthStr::width(self.0)
    }
}

impl<'a> From<Grapheme<'a>> for &'a str {
    fn from(g: Grapheme<'a>) -> Self {
        g.0
    }
}

impl<'a> AsRef<str> for Grapheme<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> std::ops::Deref for Grapheme<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Graphemes<'a> {
    graphemes: unicode_segmentation::Graphemes<'a>,
}

impl<'a> Graphemes<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            graphemes: unicode_segmentation::UnicodeSegmentation::graphemes(s, true),
        }
    }
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = Grapheme<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.graphemes.next().map(|g| Grapheme(g))
    }
}

impl<'a> DoubleEndedIterator for Graphemes<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.graphemes.next_back().map(|g| Grapheme(g))
    }
}

#[derive(Debug)]
pub struct GraphemeIndices<'a> {
    grapheme_indices: unicode_segmentation::GraphemeIndices<'a>,
}

impl<'a> GraphemeIndices<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            grapheme_indices: unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true),
        }
    }
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, Grapheme<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.grapheme_indices.next().map(|(i, g)| (i, Grapheme(g)))
    }
}

impl<'a> DoubleEndedIterator for GraphemeIndices<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.grapheme_indices
            .next_back()
            .map(|(i, g)| (i, Grapheme(g)))
    }
}

// TODO: Move to terminal module.
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
        let Some((start, g)) = self.graphemes.next() else {
            return None;
        };

        if g.equals("\x1b") {
            match self.graphemes.find(|(_, g)| g.equals("m")) {
                Some((end, _)) => Some(GraphemeOrAnsi::Ansi(&self.text[start..end + 1])),
                None => None,
            }
        } else {
            Some(GraphemeOrAnsi::Grapheme(g))
        }
    }
}

pub struct WordsIter<'a> {
    text: &'a str,
    graphemes: GraphemeIndices<'a>,
    event: Option<(usize, &'a str, WordsEvent)>,
    word: Option<usize>,
}

#[derive(Debug)]
pub enum WordsEvent {
    Newline,
    Whitespace,
    Word,
}

impl<'a> WordsIter<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            graphemes: GraphemeIndices::new(text),
            event: None,
            word: None,
        }
    }
}

impl<'a> Iterator for WordsIter<'a> {
    type Item = (usize, &'a str, WordsEvent);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.event.take() {
            return Some(event);
        }

        while let Some((i, g)) = self.graphemes.next() {
            if g.is_whitespace() {
                let next_event = if g.is_newline() {
                    WordsEvent::Newline
                } else {
                    WordsEvent::Whitespace
                };

                if let Some(ws) = self.word.take() {
                    self.event = Some((i, g.0, next_event));
                    return Some((ws, &self.text[ws..i], WordsEvent::Word));
                } else {
                    return Some((i, g.0, next_event));
                }
            } else {
                if self.word.is_none() {
                    self.word = Some(i);
                }
            }
        }

        self.word
            .take()
            .map(|i| (i, &self.text[i..], WordsEvent::Word))
    }
}

#[macro_export]
macro_rules! repeat {
    ($s:expr, 0) => {{
        const _: &str = $s;
        ""
    }};
    ($s:expr, 1) => {{
        const _: &str = $s;
        $s
    }};
    ($s:expr, 2) => {{
        const _: &str = $s;
        utils::concat!($s, $s)
    }};
    ($s:expr, 3) => {{
        const _: &str = $s;
        utils::concat!($s, $s, $s)
    }};
    ($s:expr, 4) => {{
        const _: &str = $s;
        utils::concat!($s, $s, $s, $s)
    }};
    ($s:expr, 5) => {{
        const _: &str = $s;
        utils::concat!($s, $s, $s, $s, $s)
    }};
}

pub use constcat::concat;
