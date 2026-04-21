#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to construct duration: {0}")]
    PeriodOutOfBounds(String),
    #[error("failed to run analytics")]
    TradeExpired(String),
    #[error("invalid market data: {0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, Error>;
