use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Json parsing error: {0}")]
    JsonParsingError(#[from] serde_json::Error),
    // Todo get this as a from anki error
    #[error("Anki lib error")]
    AnkiError,
    #[error("Zip parsing error: {0}")]
    ZipParsingError(#[from] anki::media::sync::zip::result::ZipError),
    #[error("Actix web error: {0}")]
    ActixError(#[from] actix_web::Error),
    #[error("Utf8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Value error: {0}")]
    ValueNotFound(String),
    #[error("Unknown data user error")]
    Unknown,
}
