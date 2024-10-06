mod deco_one;
mod window;
mod windows;

pub mod utils;

use std::fmt::{Display, Formatter};
pub use window::*;
pub use windows::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidHandle,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
