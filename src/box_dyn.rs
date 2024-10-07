use crate::{Window, WindowState, WindowUserState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::TypeId;

pub type DynUserState = Box<dyn WindowUserState + 'static>;
pub type DynWindow = Box<dyn Window<DynUserState> + 'static>;

impl Window<DynUserState> for DynWindow {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }

    fn render_ref(&self, area: Rect, buf: &mut Buffer, user: &mut DynUserState) {
        self.as_ref().render_ref(area, buf, user);
    }
}

impl WindowUserState for DynUserState {
    fn window_state(&self) -> &WindowState {
        self.as_ref().window_state()
    }

    fn window_state_mut(&mut self) -> &mut WindowState {
        self.as_mut().window_state_mut()
    }
}
