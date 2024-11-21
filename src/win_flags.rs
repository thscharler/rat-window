use crate::_private::NonExhaustive;
use rat_focus::FocusFlag;

/// Window parameters.
///
/// These represent some  state for each window and is
/// updated for each render by calling [WinState::get_flags()].
///
#[derive(Debug, Clone)]
pub struct WinFlags {
    /// Window title.
    pub title: String,

    /// Modal window.
    /// Blocks most window operations for other windows.
    /// This is the flag for dialog-like windows.
    pub modal: bool,
    /// Window can be closed.
    pub closeable: bool,
    /// Window can be resized.
    pub resizable: bool,
    /// Window can be moved.
    pub moveable: bool,

    /// Focused window.
    ///
    /// This is not used with a [rat_focus::Focus] instance.
    /// The window manager has its own focus logic instead.
    pub focus: FocusFlag,

    pub non_exhaustive: NonExhaustive,
}

impl Default for WinFlags {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            modal: false,
            closeable: false,
            resizable: true,
            moveable: true,
            focus: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}
