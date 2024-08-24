//! Observer patterns.
//!     Observer is subscribed to Observable events.

use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

// Define the Observer trait with an update method
pub trait Observer: Debug {
    fn update(&mut self, observable: &dyn Observable);
}

// Define the Observable trait with methods to register, remove, and notify observers
pub trait Observable {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>);
    fn detach(&mut self, observer: Rc<RefCell<dyn Observer>>);
    fn notify_observers(&self);
}

// Helper trait for downcasting
pub trait Downcast: Observable {
    fn downcast_ref<T: Observable>(&self) -> Option<&T> {
        if self.as_any().is::<T>() {
            Some(unsafe { &*(self as *const dyn Observable as *const T) })
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Observable + 'static> Downcast for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}