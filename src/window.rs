use crate::WindowState;
use log::debug;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::{Any, TypeId};
use std::fmt::Debug;

/// Trait for a window.
pub trait Window<U>: Any
where
    U: WindowUserState,
{
    /// Return the type-id of a compatible WindowUserState.
    fn state_id(&self) -> TypeId;

    /// Render
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        win_state: &mut WindowState,
        win_user: &mut U,
    );
}

pub trait WindowUserState: Any {
    // no extras
}

impl<U> dyn Window<U>
where
    U: WindowUserState,
{
    /// down cast Any style.
    pub fn downcast_ref<R: Window<U>>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn Window<U> = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: Window<U>>(&'_ mut self) -> Option<&'_ mut R> {
        if (&*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn Window<U> = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl<U> Window<U> for Box<dyn Window<U> + 'static>
where
    U: WindowUserState,
{
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut WindowState, user: &mut U) {
        self.as_ref().render_ref(area, buf, state, user);
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
    pub fn downcast_mut<R: WindowUserState>(&mut self) -> &mut R {
        if (&*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WindowUserState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}
