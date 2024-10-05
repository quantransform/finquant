//! Observer patterns.
//!     Observer is subscribed to Observable events.

use crate::error::Result;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>);
    fn notify_observers(&self) -> Result<()>;
    fn as_any(&self) -> &dyn Any;
}

// Define the Observer trait with an update method
pub trait Observer {
    fn update(&mut self, observable: &dyn Observable) -> Result<()>;
    fn as_any(&self) -> &dyn Any;
}
