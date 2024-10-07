use crate::WindowState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::{Any, TypeId};

/// Trait for a window.
pub trait Window<U>: Any
where
    U: WindowUserState,
{
    /// Return the type-id of a compatible WindowUserState.
    fn state_id(&self) -> TypeId;

    /// Render
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut U);
}

pub trait WindowUserState: Any {
    /// Access to the window state stored in the user state.
    fn window_state(&self) -> &WindowState;

    /// Access to the window state stored in the user state.
    fn window_state_mut(&mut self) -> &mut WindowState;
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

impl dyn WindowUserState {
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
