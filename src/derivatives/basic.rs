use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum Direction {
    Buy = 1,
    Sell = -1,
}
#[derive(Deserialize, Serialize, Debug)]
pub enum Style {
    FXForward,
    FXCall,
    FXPut,
    IRSwap,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct BasicInfo {
    pub trade_date: NaiveDate,
    pub style: Style,
    pub direction: Direction,
    pub expiry_date: NaiveDate,
    pub delivery_date: NaiveDate,
}

pub trait Derivatives {
    fn pricer(&self) -> f64;
}
