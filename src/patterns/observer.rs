//! Observer patterns.
//!     Observer is subscribed to Observable events.

use crate::error::Result;

// Define the Observer trait with an update method
#[allow(dead_code)]
pub trait Observer {
    fn update(&mut self) -> Result<()>;
}

// Define the Observable trait with methods to register, remove, and notify observers
#[allow(dead_code)]
pub trait Observable<'a, T: Observer> {
    fn attach(&mut self, observer: &'a T);
    fn detach(&mut self, observer: &'a T);
    fn notify_observers(&self);
}
