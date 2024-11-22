mod deco_one;
mod deco_two;
mod util;
mod win;
mod win_ct;
mod win_flags;
mod window_manager;
mod windows;

pub use deco_one::*;
pub use deco_two::*;
pub use win::*;
pub use win_ct::*;
pub use win_flags::*;
pub use window_manager::*;
pub use windows::*;

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
