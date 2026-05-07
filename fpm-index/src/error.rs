use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("http error for {url}: {source}")]
    Http { url: String, source: reqwest::Error },

    #[error("msgpack decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),

    #[error("msgpack encode error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),

    #[error("delta apply error: {0}")]
    Delta(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("db error: {0}")]
    Db(#[from] fpm_db::error::DbError),

    #[error("{0}")]
    Other(String),
}
