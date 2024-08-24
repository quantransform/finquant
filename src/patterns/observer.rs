//! Observer patterns.
//!     Observer is subscribed to Observable events.

use std::any::Any;
use std::fmt::Debug;

// Define the Observer trait with an update method
pub trait Observer: Debug {
    fn update(&self, event: &dyn Any);
}

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable: Debug + Any {
    fn attach(&mut self, observer: Box<dyn Observer>);
    fn detach(&mut self, observer: &dyn Observer);
    fn notify_observers(&self);
}
