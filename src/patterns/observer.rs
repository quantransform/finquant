//! Observer patterns.
//!     Observer is subscribed to Observable events.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Deserialize, Serialize, Debug)]
pub enum StatusEnum {
    Initialised,
    Updated { version: i32 },
}

// Define the Observer trait with an update method
#[typetag::serde(tag = "type")]
pub trait Observer: Debug {
    fn update(&self, observable: &dyn Observable) -> Result<()>;
}

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable {
    fn attach(&mut self, observer: Box<dyn Observer>);
    fn notify_observers(&self);
    fn get_status(&self) -> &StatusEnum;
}
