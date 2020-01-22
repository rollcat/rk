use std::boxed::Box;
use std::error::Error;
use std::result::Result;

pub type DynResult<T> = Result<T, Box<dyn Error>>;
