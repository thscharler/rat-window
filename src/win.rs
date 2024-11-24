use crate::win_base::WinBaseState;
use crate::window_manager::{relocate_event, WindowManager};
use crate::windows::WindowsState;
use crate::{render_windows, Windows};
use rat_event::{HandleEvent, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::ops::Deref;

///
/// Trait for rendering the contents of a widget.
///
/// TODO: change to StatefulWidgetRef once #1505 is released.
///
pub trait WinWidget: Debug {
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinState);
}

///
/// State for a window.
///
pub trait WinState: WinBaseState + Any + Debug {}

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

impl<'a, M> StatefulWidget for Windows<'a, dyn WinState, M>
where
    M: WindowManager + 'a + Debug,
    M::State: Debug,
{
    type State = WindowsState<dyn WinWidget, dyn WinState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinWidget, dyn WinState, M>,
    ) {
        _ = render_windows(
            &self,
            |window, widget_area, buf, window_state| {
                window.render_ref(widget_area, buf, window_state);
                Ok::<(), ()>(())
            },
            area,
            buf,
            state,
        );
    }
}

impl<T, M> HandleEvent<crossterm::event::Event, Regular, Outcome>
    for WindowsState<T, dyn WinState, M>
where
    T: WinWidget + ?Sized + 'static + Debug,
    M: WindowManager + Debug,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome> + Debug,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let Some(event) = relocate_event(self.rc.manager.borrow().deref(), event) else {
            return Outcome::Continue;
        };

        // forward to window-manager
        self.rc.manager.borrow_mut().handle(event.as_ref(), Regular)
    }
}
