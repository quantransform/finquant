use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Underlying {
    EURUSD,
    GBPUSD,
    EURGBP,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Currency {
    GBP,
    EUR,
    USD,
}

pub trait FXDerivatives {
    fn delta(&self) -> f32;
    fn gamma(&self) -> f32;
    fn vega(&self) -> f32;
}
