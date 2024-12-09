use crate::event::WindowsOutcome;
use crate::window_manager::WindowManager;
use crate::{render_windows, WindowManagerState, WindowMode, Windows, WindowsState};
use rat_cursor::HasScreenCursor;
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer};
use rat_reloc::RelocatableState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{type_name, Any, TypeId};
use std::fmt::Debug;

/// Trait for a window with event handling.
pub trait WinCtWidget {
    type State: WinCtState + ?Sized;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}

///
/// Trait for a window with event handling.
///
pub trait WinCtState: RelocatableState + HasScreenCursor + Any
where
    Self: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
    /// Cast the window as a FocusContainer for focus handling.
    fn as_focus_container(&self) -> &dyn FocusContainer;
}

impl dyn WinCtState {
    /// Call the closure for a given window.
    pub fn for_ref<S: WinCtState + 'static>(&self, f: impl FnOnce(&S)) {
        let downcast = self.downcast_ref::<S>().expect(type_name::<S>());
        f(downcast)
    }

    /// Call the closure for a given window.
    pub fn for_mut<S: WinCtState + 'static>(&mut self, f: impl FnOnce(&mut S)) {
        let downcast = self.downcast_mut::<S>().expect(type_name::<S>());
        f(downcast)
    }

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
    type State = WindowsState<dyn WinCtWidget<State = dyn WinCtState>, dyn WinCtState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinCtWidget<State = dyn WinCtState>, dyn WinCtState, M>,
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

impl<T, M> HasScreenCursor for WindowsState<T, dyn WinCtState, M>
where
    T: WinCtWidget + ?Sized + 'static,
    M: WindowManager + Debug,
{
    fn screen_cursor(&self) -> Option<(u16, u16)> {
        // only have the windows themselves.
        let manager = self.rc.manager.borrow();
        if manager.mode() == WindowMode::Config {
            None
        } else {
            if let Some(handle) = manager.front_window() {
                let window_state = self.window_state(handle);
                let window_state = window_state.borrow();
                window_state.screen_cursor()
            } else {
                None
            }
        }
    }
}

impl<T, M> FocusContainer for WindowsState<T, dyn WinCtState, M>
where
    T: WinCtWidget + ?Sized + 'static,
    M: WindowManager + Debug,
{
    fn build(&self, builder: &mut FocusBuilder) {
        // only have the windows themselves.
        let manager = self.rc.manager.borrow();

        if manager.mode() == WindowMode::Config {
            for handle in self.handles_create() {
                let frame = manager.window_frame(handle);
                // only add the window as widget
                builder.widget(frame.as_has_focus());
            }
        } else {
            for handle in self.handles_create().into_iter() {
                let frame = manager.window_frame(handle);
                let frame_container = frame.as_focus_container();
                let window_state = self.window_state(handle);

                let tag = builder.start(
                    frame_container.container(),
                    frame_container.area(),
                    frame_container.area_z(),
                );
                builder.container(window_state.borrow().as_focus_container());
                builder.end(tag);
            }
        }
    }

    fn container(&self) -> Option<ContainerFlag> {
        // container for all windows
        Some(self.rc.manager.borrow().container())
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl<T, M> HandleEvent<crossterm::event::Event, Regular, WindowsOutcome>
    for WindowsState<T, dyn WinCtState, M>
where
    T: WinCtWidget + ?Sized + 'static,
    M::Outcome: ConsumedEvent + Into<WindowsOutcome>,
    M: WindowManager + Debug,
    M::State: HandleEvent<crossterm::event::Event, Regular, M::Outcome> + Debug,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> WindowsOutcome {
        // Special action for focus.
        self.rc.manager.borrow_mut().focus_to_front();

        // forward to window-manager
        let r = self.rc.manager.borrow_mut().handle(event, Regular).into();

        let r = r.or_else(|| {
            // forward to all windows
            'f: {
                for handle in self.handles_render().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |_window, window_state| {
                        window_state.handle(event, Regular)
                    });
                    if r.is_consumed() {
                        break 'f r.into();
                    }
                }
                WindowsOutcome::Continue
            }
        });

        r
    }
}
