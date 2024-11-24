use crate::win_base::WinBaseState;
use crate::win_flags::WinFlags;
use crate::window_manager::{relocate_event, WindowManager};
use crate::windows::WindowsState;
use crate::{WindowManagerState, Windows};
use rat_event::{HandleEvent, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{Any, TypeId};
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
pub trait WinState: WinBaseState + Any {
    /// Get a copy of the windows flags governing general
    /// behaviour of the window.
    fn get_flags(&self) -> WinFlags;

    /// Return self as dyn WinState.
    fn as_dyn(&mut self) -> &mut dyn WinState;
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

impl<'a, M> StatefulWidget for Windows<'a, dyn WinState, M>
where
    M: WindowManager + 'a,
{
    type State = WindowsState<dyn WinWidget, dyn WinState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinWidget, dyn WinState, M>,
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
                window.render_ref(widget_area, &mut tmp_buf, window_state.as_dyn());

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
    for WindowsState<T, dyn WinState, M>
where
    T: WinWidget + ?Sized + 'static,
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let Some(event) = relocate_event(self.rc.manager.borrow().deref(), event) else {
            return Outcome::Continue;
        };

        // forward to window-manager
        self.rc.manager.borrow_mut().handle(event.as_ref(), Regular)
    }
}
