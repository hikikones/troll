use std::{error::Error, fmt::Display, range::Range};

pub trait StringExt {
    fn slice(&self, range: Range<usize>) -> &str;
    fn push_str_get_range(&mut self, s: &str) -> Range<usize>;
    fn push_str_get_range2(&mut self, s1: &str, s2: &str) -> (Range<usize>, Range<usize>);
    fn push_fmt(&mut self, content: impl Display);
    fn push_fmt_get_range(&mut self, content: impl Display) -> Range<usize>;
    fn extend_get_range<I>(&mut self, iter: I) -> Range<usize>
    where
        I: IntoIterator,
        String: Extend<I::Item>;
}

impl StringExt for String {
    fn slice(&self, range: Range<usize>) -> &str {
        &self[range]
    }

    fn push_str_get_range(&mut self, s: &str) -> Range<usize> {
        let start = self.len();
        self.push_str(s);

        Range {
            start,
            end: self.len(),
        }
    }

    fn push_str_get_range2(&mut self, s1: &str, s2: &str) -> (Range<usize>, Range<usize>) {
        let start = self.len();
        self.push_str(s1);
        let middle = self.len();
        self.push_str(s2);

        (
            Range { start, end: middle },
            Range {
                start: middle,
                end: self.len(),
            },
        )
    }

    fn push_fmt(&mut self, content: impl Display) {
        let _ = std::fmt::Write::write_fmt(self, format_args!("{content}"));
    }

    fn push_fmt_get_range(&mut self, content: impl Display) -> Range<usize> {
        let start = self.len();
        let _ = std::fmt::Write::write_fmt(self, format_args!("{content}"));

        Range {
            start,
            end: self.len(),
        }
    }

    fn extend_get_range<I>(&mut self, iter: I) -> Range<usize>
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        let start = self.len();
        self.extend(iter);

        Range {
            start,
            end: self.len(),
        }
    }
}

#[derive(Debug)]
pub struct ContextError<E, C> {
    context: C,
    source: E,
}

impl<E, C> Display for ContextError<E, C>
where
    E: Display,
    C: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl<E, C> Error for ContextError<E, C>
where
    E: Error + 'static,
    C: Display + std::fmt::Debug,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub trait ResultExt<T, E> {
    fn context(self, context: &'static str) -> Result<T, ContextError<E, &'static str>>;

    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<E, C>>
    where
        C: Display + std::fmt::Debug,
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn context(self, context: &'static str) -> Result<T, ContextError<E, &'static str>> {
        self.map_err(|source| ContextError { context, source })
    }

    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<E, C>>
    where
        C: Display + std::fmt::Debug,
        F: FnOnce() -> C,
    {
        self.map_err(|source| ContextError {
            context: context(),
            source,
        })
    }
}
