macro_rules! const_unwrap {
    ($expr:expr) => {
        match $expr {
            Some(val) => val,
            None => panic!("expected some value"),
        }
    };
}

pub(crate) use const_unwrap;
