use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("ScyllaDB error: {0}")]
    Scylla(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}
