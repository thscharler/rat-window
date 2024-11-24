use crate::win_base::WinBaseState;
use crate::window_manager::{relocate_event, WindowManager};
use crate::{render_windows, Windows, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::ops::Deref;

/// Trait for a window with event handling.
pub trait WinCtWidget: Debug {
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinCtState);
}

///
/// Trait for a window with event handling.
///
/// Reuses [WinState] and adds event handling.
///
pub trait WinCtState: WinBaseState + Any + Debug
where
    Self: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
}

impl dyn WinCtState {
    /// down cast Any style.
    pub fn downcast_ref<R: WinCtState + 'static>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WinCtState = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: WinCtState + 'static>(&'_ mut self) -> Option<&'_ mut R> {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WinCtState = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl<'a, M> StatefulWidget for Windows<'a, dyn WinCtState, M>
where
    M: WindowManager + 'a + Debug,
    M::State: Debug,
{
    type State = WindowsState<dyn WinCtWidget, dyn WinCtState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinCtWidget, dyn WinCtState, M>,
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
    for WindowsState<T, dyn WinCtState, M>
where
    T: WinCtWidget + ?Sized + 'static + Debug,
    M: WindowManager + Debug,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome> + Debug,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let Some(relocated) = relocate_event(self.rc.manager.borrow().deref(), event) else {
            return Outcome::Continue;
        };

        // forward to window-manager
        let r = self
            .rc
            .manager
            .borrow_mut()
            .handle(relocated.as_ref(), Regular);

        let r = r.or_else(|| {
            // forward to all windows
            'f: {
                for handle in self.handles().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |_window, window_state| {
                        window_state.handle(relocated.as_ref(), Regular)
                    });
                    if r.is_consumed() {
                        break 'f r;
                    }
                }
                Outcome::Continue
            }
        });

        r
    }
}
