pub mod docx;
pub mod pptx;

use crate::ir::Document;
use std::io::{self, Write, Seek};

pub trait Backend {
    fn compile<W: Write + Seek>(&self, document: &Document, writer: W) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Zip(zip::result::ZipError),
    Xml(quick_xml::Error),
    MissingImage(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e)
    }
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error::Xml(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Zip(e) => write!(f, "ZIP error: {}", e),
            Error::Xml(e) => write!(f, "XML error: {}", e),
            Error::MissingImage(p) => write!(f, "Missing image file: {}", p),
        }
    }
}
