mod deco_one;
pub mod event;
mod util;
mod win;
mod win_ct;
mod win_flags;
mod win_salsa;
mod window_manager;
mod windows;

pub use deco_one::*;
pub use util::*;
pub use win::*;
pub use win_ct::*;
pub use win_flags::*;
pub use win_salsa::*;
pub use window_manager::*;
pub use windows::*;

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
