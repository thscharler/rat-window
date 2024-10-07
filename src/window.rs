use crate::WindowState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Trait for a window.
pub trait Window:
    StatefulWidgetRef<State = (Rc<RefCell<WindowState>>, Rc<RefCell<dyn WindowUserState>>)> + Any
{
    /// Return the type-id of a compatible WindowUserState.
    fn state_id(&self) -> TypeId;
}

pub trait WindowUserState: Any {
    // no extras
}

impl dyn Window {
    /// down cast Any style.
    pub fn downcast_ref<R: Window>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn Window = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: Window>(&'_ mut self) -> Option<&'_ mut R> {
        if (&*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn Window = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl Window for Box<dyn Window + 'static> {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }
}

impl StatefulWidgetRef for Box<dyn Window + 'static> {
    type State = (Rc<RefCell<WindowState>>, Rc<RefCell<dyn WindowUserState>>);

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.as_ref().render_ref(area, buf, state);
    }
}

impl WindowUserState for Box<dyn WindowUserState + 'static> {}

impl<T: WindowUserState> WindowUserState for Box<T> {}

impl WindowUserState for () {}

impl dyn WindowUserState {
    /// down cast for Box<dyn WindowUserState
    pub fn downcast_box_dyn<R: WindowUserState>(&self) -> &R {
        let first = self.downcast_ref::<Box<dyn WindowUserState>>();
        first.as_ref().downcast_ref::<R>()
    }

    /// down cast for Box<dyn WindowUserState
    pub fn downcast_box_dyn_mut<R: WindowUserState>(&mut self) -> &mut R {
        let first = self.downcast_mut::<Box<dyn WindowUserState>>();
        first.as_mut().downcast_mut::<R>()
    }

    /// down cast Any style.
    pub fn downcast_ref<R: WindowUserState>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowUserState = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: Any>(&mut self) -> &mut R {
        if (&*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WindowUserState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}
