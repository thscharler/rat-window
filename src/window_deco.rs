use crate::WindowState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::{Any, TypeId};

/// Trait for a widget that renders the window frame.
pub trait WindowDeco: Any {
    /// Return the type-id of a compatible WindowFrameStyle.
    fn style_id(&self) -> TypeId;

    /// Draws the current state of the widget in the given buffer. That is the only method required
    /// to implement a custom stateful widget.
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        style: Option<&dyn WindowDecoStyle>,
        state: &mut dyn WindowState,
    );
}

/// Style parameters for a window frame.
pub trait WindowDecoStyle: Any {}

impl dyn WindowDecoStyle {
    /// down cast Any style.
    pub fn downcast_ref<R: WindowDecoStyle>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowDecoStyle = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }
}

impl dyn WindowDeco {
    /// down cast Any style.
    pub fn downcast_ref<R: WindowDecoStyle>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowDeco = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }
}
