use thiserror::Error;
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Json parsing error: {0}")]
    JsonParsing(#[from] serde_json::Error),
    /// https://github.com/ankicommunity/anki-sync-server-rs/issues/40
    #[error("Anki lib error {0}")]
    AnkiError(#[from] anki::error::AnkiError),
    #[error("Anki lib fileio error {0}")]
    AnkiFileIoError(#[from] anki::error::FileIoError),
    #[error("Zip parsing error: {0}")]
    ZipParsing(#[from] zip::result::ZipError),
    #[error("Actix web error: {0}")]
    Actix(#[from] actix_web::Error),
    #[cfg(feature = "tls")]
    #[error("Rustls error: {0}")]
    Rustls(#[from] rustls::Error),
    #[error("Utf8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Value error: {0}")]
    ValueNotFound(String),
    #[error("ParseConfig error: {0}")]
    ParseConfig(String),
    #[error("Unknown data user error")]
    Unknown,
    #[error(transparent)]
    UserError(#[from] crate::user::UserError),
    #[error("Error while serializing data: {0}")]
    SerdeTomlSerializingError(#[from] toml::ser::Error),
    #[error("Error while deserializing data: {0}")]
    SerdeTomlDeserializingError(#[from] toml::de::Error),
    #[error("session error: {0}")]
    SessionError(String),
    #[error("Error while paring GET request: {0}")]
    ParseGET(String),
    #[error("Error while paring multipart stream: {0}")]
    Multipart(#[from] actix_multipart::MultipartError),
}
