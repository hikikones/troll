use utils::{Grapheme, GraphemeIndices, grapheme_indices};

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
