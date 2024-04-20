//! Observer patterns.

// Define the Observer trait with an update method
pub trait Observer {
    fn update(&self);
}

// Define the Subject trait with methods to register, remove, and notify observers
pub trait Subject<'a, T: Observer> {
    fn attach(&mut self, observer: &'a T);
    fn detach(&mut self, observer: &'a T);
    fn notify_observers(&self);
}
