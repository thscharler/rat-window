use crate::WindowState;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// Trait for a widget that renders the window frame.
pub trait WindowDeco:
    StatefulWidgetRef<State = (Rc<RefCell<WindowState>>, Rc<dyn WindowDecoStyle>)> + Any
{
    /// Return the type-id of a compatible WindowFrameStyle.
    fn style_id(&self) -> TypeId;
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
