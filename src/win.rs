use crate::event::WindowsOutcome;
use crate::window_manager::{relocate_event, WindowManager};
use crate::windows::WindowsState;
use crate::{render_windows, WindowManagerState, Windows};
use rat_event::{ConsumedEvent, HandleEvent, Regular};
use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use std::any::{type_name, Any, TypeId};
use std::fmt::Debug;
use std::ops::Deref;

///
/// Trait for rendering the contents of a widget.
///
/// TODO: change to StatefulWidgetRef once #1505 is released.
///
pub trait WinWidget {
    type State: WinState + ?Sized;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}

///
/// State for a window.
///
pub trait WinState: Any {}

impl dyn WinState {
    /// Call the closure for a given window.
    pub fn for_ref<S: WinState + 'static>(&self, f: impl FnOnce(&S)) {
        let downcast = self.downcast_ref::<S>().expect(type_name::<S>());
        f(downcast)
    }

    /// Call the closure for a given window.
    pub fn for_mut<S: WinState + 'static>(&mut self, f: impl FnOnce(&mut S)) {
        let downcast = self.downcast_mut::<S>().expect(type_name::<S>());
        f(downcast)
    }

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
    type State = WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, M>;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, M>,
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

impl<M> FocusContainer for WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, M>
where
    M: WindowManager,
{
    fn build(&self, builder: &mut FocusBuilder) {
        // only have the windows themselves.
        let manager = self.rc.manager.borrow();

        // navigate the tabs in creation order.
        for handle in self.handles_create() {
            let frame = manager.window_frame(handle);
            let has_focus = frame.as_has_focus();

            let container_end =
                builder.start(Some(manager.window_container(handle)), has_focus.area());
            builder.widget(has_focus);
            builder.end(container_end);
        }
    }

    fn container(&self) -> Option<ContainerFlag> {
        Some(self.rc.manager.borrow().container())
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl<M> HandleEvent<crossterm::event::Event, Regular, WindowsOutcome>
    for WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, M>
where
    M: WindowManager + Debug,
    M::Outcome: ConsumedEvent + Into<WindowsOutcome>,
    M::State: HandleEvent<crossterm::event::Event, Regular, M::Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> WindowsOutcome {
        let Some(event) = relocate_event(self.rc.manager.borrow().deref(), event) else {
            return WindowsOutcome::Continue;
        };

        // Special action for focus.
        self.rc.manager.borrow_mut().focus_to_front();

        // forward to window-manager
        self.rc
            .manager
            .borrow_mut()
            .handle(event.as_ref(), Regular)
            .into()
    }
}
