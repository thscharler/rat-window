use crate::WindowState;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// Trait for a widget that renders the window frame.
pub trait WindowFrame:
    StatefulWidgetRef<State = (Rc<RefCell<WindowState>>, Rc<dyn WindowFrameStyle>)> + Any
{
}

/// Style parameters for a window frame.
pub trait WindowFrameStyle: Any {}

impl dyn WindowFrameStyle {
    /// down cast Any style.
    pub fn downcast_ref<R: 'static>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowFrameStyle = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }
}

impl dyn WindowFrame {
    /// down cast Any style.
    pub fn downcast_ref<R: 'static>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowFrame = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }
}
