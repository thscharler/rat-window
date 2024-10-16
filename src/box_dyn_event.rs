//!
//! Defines useful types for dyn windows.
//!
//! This here is for view+event handling windows.
//!
use crate::{Window, WindowState, WindowSysState};
use rat_event::{HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{FocusBuilder, FocusContainer};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidgetRef;
use std::any::TypeId;

/// User state for widgets with event-handling.
pub trait EventUserState:
    WindowState
    + FocusContainer
    + HandleEvent<crossterm::event::Event, Regular, Outcome>
    + HandleEvent<crossterm::event::Event, MouseOnly, Outcome>
{
}

/// User state for dyn widgets with event-handling.
pub type DynEventUserState = Box<dyn EventUserState + 'static>;
/// Widget type for dyn widgets with event-handling.
pub type DynEventWindow = Box<dyn Window<State = DynEventUserState> + 'static>;

impl Window for DynEventWindow {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }
}

impl StatefulWidgetRef for DynEventWindow {
    type State = DynEventUserState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.as_ref().render_ref(area, buf, state);
    }
}

impl WindowState for DynEventUserState {
    fn boxed_type_id(&self) -> TypeId {
        self.as_ref().type_id()
    }

    fn window(&self) -> &WindowSysState {
        self.as_ref().window()
    }

    fn window_mut(&mut self) -> &mut WindowSysState {
        self.as_mut().window_mut()
    }
}

impl EventUserState for DynEventUserState {}

impl FocusContainer for DynEventUserState {
    fn build(&self, builder: &mut FocusBuilder) {
        self.as_ref().build(builder);
    }
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for DynEventUserState {
    fn handle(&mut self, event: &crossterm::event::Event, qualifier: Regular) -> Outcome {
        self.as_mut().handle(event, qualifier)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for DynEventUserState {
    fn handle(&mut self, event: &crossterm::event::Event, qualifier: MouseOnly) -> Outcome {
        self.as_mut().handle(event, qualifier)
    }
}

impl dyn EventUserState {
    /// down cast Any style.
    pub fn downcast_ref<R: EventUserState>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn EventUserState = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: EventUserState>(&mut self) -> &mut R {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn EventUserState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}
