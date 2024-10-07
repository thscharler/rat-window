use crate::{Window, WindowState, WindowUserState};
use rat_event::{HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{FocusBuilder, HasFocus};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::TypeId;

/// For UserState with event handling.
pub trait EventUserState:
    WindowUserState
    + HasFocus
    + HandleEvent<crossterm::event::Event, Regular, Outcome>
    + HandleEvent<crossterm::event::Event, MouseOnly, Outcome>
{
}

pub type DynEventUserState = Box<dyn EventUserState + 'static>;
pub type DynEventWindow = Box<dyn Window<DynEventUserState> + 'static>;

impl EventUserState for () {}

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
        if (&*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn EventUserState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}

impl Window<DynEventUserState> for DynEventWindow {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut WindowState,
        user: &mut DynEventUserState,
    ) {
        self.as_ref().render_ref(area, buf, state, user);
    }
}

impl WindowUserState for DynEventUserState {}
impl EventUserState for DynEventUserState {}

impl HasFocus for DynEventUserState {
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
