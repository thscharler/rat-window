use std::fmt::{Display, Formatter};

mod deco_layout;
mod deco_one;
mod window;
mod window_builder;
mod window_deco;
mod window_state;
mod windows;

pub use window::*;
pub use window_builder::*;
pub use window_state::*;
pub use windows::*;

pub mod box_dyn;
pub mod box_dyn_event;
pub mod utils;

/// Window decorations.
///
/// There is currently One.
pub mod deco {
    use crate::{deco_layout, deco_one};

    pub use deco_layout::*;
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
