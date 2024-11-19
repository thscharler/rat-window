use crate::_private::NonExhaustive;
use rat_focus::FocusFlag;

#[derive(Debug, Clone)]
pub struct WinFlags {
    pub title: String,

    pub modal: bool,
    pub closeable: bool,
    pub resizable: bool,
    pub moveable: bool,

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
