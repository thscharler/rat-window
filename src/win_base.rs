use crate::{WinFlags, WinHandle};

pub trait WinBaseState {
    /// Set the handle used for this window.
    fn set_handle(&mut self, handle: WinHandle);

    /// Get a copy of the windows flags governing general
    /// behaviour of the window.
    fn get_flags(&self) -> WinFlags;
}
