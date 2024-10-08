use crate::WindowSysState;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};

/// Trait for a window.
///
/// That's a StatefulWidgetRef with Any added.
/// It constraints the state type to something useful.
///
/// It adds state_id for dynamic checks.
pub trait Window: Any + StatefulWidgetRef
where
    <Self as StatefulWidgetRef>::State: WindowState,
{
    /// Return the type-id of a compatible WindowUserState.
    fn state_id(&self) -> TypeId;
}

pub trait WindowState: Any {
    /// Effective type-id. Forwards to the boxed type for `Box<dyn T>`.
    fn boxed_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Access to the window state stored in the user state.
    fn window(&self) -> &WindowSysState;

    /// Access to the window state stored in the user state.
    fn window_mut(&mut self) -> &mut WindowSysState;
}

impl<U: WindowState> dyn Window<State = U> {
    /// down cast Any style.
    pub fn downcast_ref<R: Window<State = U>>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn Window<State = U> = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: Window<State = U>>(&'_ mut self) -> Option<&'_ mut R> {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn Window<State = U> = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl dyn WindowState {
    /// down cast Any style.
    pub fn downcast_ref<R: WindowState>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowState = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: WindowState>(&mut self) -> &mut R {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WindowState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}
