use std::fmt;

#[derive(Debug)]
pub enum Error {
    Http(reqwest::Error),
    HtmlParse(Vec<String>),
    MissingElement(&'static str),
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(e) => write!(f, "HTTP error: {e}"),
            Error::HtmlParse(errors) => {
                write!(f, "HTML parse errors:")?;
                for e in errors {
                    write!(f, "\n\t{e}")?;
                }
                Ok(())
            }
            Error::MissingElement(sel) => write!(f, "Document lacks element: {sel}"),
            Error::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
