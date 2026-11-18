use std::{error::Error, fmt::Display, range::Range};

pub trait StringExt {
    fn slice(&self, range: Range<usize>) -> &str;
    fn push_get_range(&mut self, ch: char) -> Range<usize>;
    fn push_repeat(&mut self, ch: char, n: usize);
    fn push_repeat_get_range(&mut self, ch: char, n: usize) -> Range<usize>;
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

    fn push_get_range(&mut self, ch: char) -> Range<usize> {
        let start = self.len();
        self.push(ch);

        Range {
            start,
            end: self.len(),
        }
    }

    fn push_repeat(&mut self, ch: char, n: usize) {
        self.extend(std::iter::repeat_n(ch, n as usize));
    }

    fn push_repeat_get_range(&mut self, ch: char, n: usize) -> Range<usize> {
        let start = self.len();
        self.push_repeat(ch, n);

        Range {
            start,
            end: self.len(),
        }
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

pub trait ResultExt<T, E> {
    fn context(self, context: &'static str) -> Result<T, ContextError<&'static str, E>>;
    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<C, E>>
    where
        C: Display + std::fmt::Debug,
        F: FnOnce() -> C;
}

pub trait ErrorExt<E> {
    fn context(self, context: &'static str) -> ContextError<&'static str, E>;
    fn with_context<C, F>(self, context: F) -> ContextError<C, E>
    where
        C: Display + std::fmt::Debug,
        F: FnOnce() -> C;
}

#[derive(Debug)]
pub struct ContextError<C, E> {
    context: C,
    source: E,
}

impl<C, E> Display for ContextError<C, E>
where
    C: Display,
    E: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl<C, E> Error for ContextError<C, E>
where
    C: Display + std::fmt::Debug,
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn context(self, context: &'static str) -> Result<T, ContextError<&'static str, E>> {
        self.map_err(|source| ContextError { context, source })
    }

    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<C, E>>
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

impl<E> ErrorExt<E> for E
where
    E: Error,
{
    fn context(self, context: &'static str) -> ContextError<&'static str, E> {
        ContextError {
            context,
            source: self,
        }
    }

    fn with_context<C, F>(self, context: F) -> ContextError<C, E>
    where
        C: Display + std::fmt::Debug,
        F: FnOnce() -> C,
    {
        ContextError {
            context: context(),
            source: self,
        }
    }
}
