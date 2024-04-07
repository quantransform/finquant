#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to construct duration: {0}")]
    PeriodOutOfBounds(String),
}

pub type Result<T> = std::result::Result<T, Error>;
