use crate::win_base::WinBaseState;
use crate::window_manager::{relocate_event, WindowManager};
use crate::{WindowManagerState, Windows, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{Any, TypeId};
use std::ops::Deref;

/// Trait for a window with event handling.
pub trait WinCtWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinCtState);
}

///
/// Trait for a window with event handling.
///
/// Reuses [WinState] and adds event handling.
///
pub trait WinCtState: WinBaseState + Any
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
    M: WindowManager + 'a,
{
    type State = WindowsState<dyn WinCtWidget, dyn WinCtState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinCtWidget, dyn WinCtState, M>,
    ) {
        state.rc.manager.borrow_mut().set_offset(self.offset);
        state.rc.manager.borrow_mut().set_area(area);

        let handles = state.rc.manager.borrow().handles();
        for handle in handles {
            state.run_for_window(handle, &mut |window, window_state| {
                self.manager.render_init_window(
                    handle,
                    window_state.get_flags(),
                    &mut state.rc.manager.borrow_mut(),
                );

                let (widget_area, mut tmp_buf) = self
                    .manager
                    .render_init_buffer(handle, &mut state.rc.manager.borrow_mut());

                // window content
                window.render_ref(widget_area, &mut tmp_buf, window_state);

                // window decorations
                self.manager.render_window_frame(
                    handle,
                    &mut tmp_buf,
                    &mut state.rc.manager.borrow_mut(),
                );

                // copy
                self.manager.render_copy_buffer(
                    &mut tmp_buf,
                    area,
                    buf,
                    &mut state.rc.manager.borrow_mut(),
                );

                // keep allocation
                self.manager
                    .render_free_buffer(tmp_buf, &mut state.rc.manager.borrow_mut());
            });
        }
    }
}

impl<T, M> HandleEvent<crossterm::event::Event, Regular, Outcome>
    for WindowsState<T, dyn WinCtState, M>
where
    T: WinCtWidget + ?Sized + 'static,
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
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
