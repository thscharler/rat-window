mod deco_one;
mod util;
mod win;
mod win_base;
mod win_ct;
mod win_flags;
// mod win_salsa;
mod window_manager;
mod windows;

pub use deco_one::*;
pub use util::*;
pub use win::*;
pub use win_base::*;
pub use win_ct::*;
pub use win_flags::*;
pub use window_manager::*;
pub use windows::*;

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
