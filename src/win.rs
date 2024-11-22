use crate::win_flags::WinFlags;
use crate::window_manager::{relocate_event, WindowManager};
use crate::windows::WindowsState;
use crate::WinHandle;
use rat_event::{HandleEvent, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::ops::Deref;

///
/// Trait for rendering the contents of a widget.
///
/// TODO: change to StatefulWidgetRef once #1505 is released.
///
pub trait WinWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinState);
}

///
/// State for a window.
///
pub trait WinState: Any + Debug {
    /// Set the handle used for this window.
    fn set_handle(&mut self, handle: WinHandle);

    /// Get a copy of the windows flags governing general
    /// behaviour of the window.
    fn get_flags(&self) -> WinFlags;

    /// Return self as dyn WinState.
    fn as_dyn(&mut self) -> &mut dyn WinState;

    /// Create the widget that renders this window.
    fn get_widget(&self) -> Box<dyn WinWidget>;
    // fn get_widget(&self) -> Box<dyn StatefulWidgetRef<State = dyn WinState>>;
}

impl dyn WinState {
    /// down cast Any style.
    pub fn downcast_ref<R: WinState + 'static>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WinState = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: WinState + 'static>(&'_ mut self) -> Option<&'_ mut R> {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WinState = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl<M> HandleEvent<crossterm::event::Event, Regular, Outcome> for &WindowsState<dyn WinState, M>
where
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let Some(event) = relocate_event(self.manager_state.borrow().deref(), event) else {
            return Outcome::Continue;
        };

        // forward to window-manager
        self.manager_state
            .borrow_mut()
            .handle(event.as_ref(), Regular)
    }
}
