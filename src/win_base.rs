use crate::WinHandle;

pub trait WinBaseState {
    /// Set the handle used for this window.
    fn set_handle(&mut self, handle: WinHandle);
}
