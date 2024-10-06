use rat_focus::{FocusFlag, HasFocusFlag};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Style;
use std::any::Any;
use std::fmt::Debug;

/// Trait for a window.
pub trait Window: Any + HasFocusFlag + Debug {
    /// Window title.
    fn title(&self) -> Option<&str> {
        Some("")
    }

    /// Can close
    fn is_closeable(&self) -> bool {
        true
    }

    /// Can resize
    fn is_resizable(&self) -> bool {
        true
    }

    /// Can move
    fn is_moveable(&self) -> bool {
        true
    }

    /// Modal?
    fn is_modal(&self) -> bool {
        false
    }

    /// Draws the current state of the widget in the given buffer. That is the only method required
    /// to implement a custom widget.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}

#[derive(Debug, Default)]
pub struct WindowState {
    ///  Window area, available after render.
    /// __read only__
    pub area: Rect,
    /// Window inner area, available after render.
    /// __read only__
    pub inner: Rect,
    ///  Window active areas, available after render.
    ///
    /// Index with WindowState consts.
    /// __read only__
    pub areas: [Rect; 11],
    /// Window modality, available after render.
    /// __read only__
    pub modal: bool,
    /// Window is closeable, available after render.
    /// __read only__
    pub closeable: bool,
    /// Window is resizable, available after render.
    /// __read only__
    pub resizable: bool,
    /// Window is moveable, available after render.
    /// __read only__
    pub moveable: bool,

    /// Perceivable styling.
    pub style: Style,

    /// Window focus.
    ///
    /// This flag is a cloned Rc from the window itself.
    pub focus: FocusFlag,
}

impl WindowState {
    pub const CLOSE: usize = 0;
    pub const MOVE: usize = 1;
    pub const RESIZE_TOP: usize = 2;
    pub const RESIZE_RIGHT: usize = 3;
    pub const RESIZE_BOTTOM: usize = 4;
    pub const RESIZE_LEFT: usize = 5;
    pub const RESIZE_TOP_LEFT: usize = 6;
    pub const RESIZE_TOP_RIGHT: usize = 7;
    pub const RESIZE_BOTTOM_RIGHT: usize = 8;
    pub const RESIZE_BOTTOM_LEFT: usize = 9;
    pub const TITLE: usize = 10;
}

impl Window for Box<dyn Window + 'static> {
    fn title(&self) -> Option<&str> {
        self.as_ref().title()
    }

    fn is_closeable(&self) -> bool {
        self.as_ref().is_closeable()
    }

    fn is_resizable(&self) -> bool {
        self.as_ref().is_resizable()
    }

    fn is_moveable(&self) -> bool {
        self.as_ref().is_moveable()
    }

    fn is_modal(&self) -> bool {
        self.as_ref().is_modal()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.as_mut().render(area, buf);
    }
}

impl HasFocusFlag for Box<dyn Window + 'static> {
    fn focus(&self) -> FocusFlag {
        self.as_ref().focus()
    }

    fn area(&self) -> Rect {
        self.as_ref().area()
    }
}
