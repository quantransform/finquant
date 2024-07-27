//! Observer patterns.
//!     Observer is subscribed to Observable events.
//!     https://medium.com/@learnwithshobhit/a-simple-implementation-of-observer-design-pattern-in-rust-fde3ef506a53

use crate::error::Result;

// Define the Observer trait with an update method
pub trait Observer {
    fn update(&mut self) -> Result<()>;
}

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable<'a, T: Observer> {
    fn attach(&mut self, observer: &'a T);
    fn detach(&mut self, observer: &'a T);
    fn notify_observers(&self);
}
