use std::fmt::{Display, Formatter};

mod deco_one;
mod window;
mod window_builder;
mod window_style;
mod windows;

pub use window::*;
pub use window_builder::WindowBuilder;
pub use windows::*;

pub mod utils;

pub mod deco {
    use crate::deco_one;
    pub use deco_one::{One, OneStyle};
}

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

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
