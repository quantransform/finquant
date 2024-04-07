/// Unwrap options at compile-time.
///
/// This is necessary as unwrap and expect on Options are not stabilised yet.
/// While this macro works in non-const contexts as well, there is no point using it
/// in that manner.
macro_rules! const_unwrap {
    ($expr:expr) => {
        match $expr {
            Some(val) => val,
            None => panic!("expected some value"),
        }
    };
}

pub(crate) use const_unwrap;
