//! Observer patterns.
//!     Observer is subscribed to Observable events.

use crate::error::Result;
use std::any::Any;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

// Define the Observer trait with an update method
pub trait Observer: Debug {
    fn update(&mut self, observable: &dyn Observable) -> Result<()>;
}

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>);
    fn notify_observers(&self);
    fn as_any(&self) -> &dyn Any;
}
