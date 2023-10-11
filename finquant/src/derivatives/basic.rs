use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Buy,
    Sell,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Style {
    FXForward,
    FXCall,
    FXPut,
    IRSwap,
}
#[derive(Serialize, Deserialize, Debug)]
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