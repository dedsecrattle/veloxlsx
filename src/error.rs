use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XlsxError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("xml parse: {0}")]
    Xml(String),
    #[error("missing zip entry: {0}")]
    MissingEntry(String),
    #[error("invalid workbook: {0}")]
    InvalidWorkbook(String),
    #[error("invalid cell reference: {0}")]
    InvalidCellRef(String),
    #[error("sheet not found: {0}")]
    SheetNotFound(String),
    #[error("invalid number: {0}")]
    InvalidNumber(String),
}

pub type Result<T> = std::result::Result<T, XlsxError>;
