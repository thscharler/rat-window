//!
//! Defines useful types for dyn windows.
//!
//! This here is for view only windows.
//!
use crate::{Window, WindowState, WindowSysState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidgetRef;
use std::any::TypeId;

/// User state for dyn widgets.
pub type DynUserState = Box<dyn WindowState + 'static>;
/// Widget type for dyn widgets.
pub type DynWindow = Box<dyn Window<State = DynUserState> + 'static>;

impl Window for DynWindow {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }
}

impl StatefulWidgetRef for DynWindow {
    type State = DynUserState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.as_ref().render_ref(area, buf, state);
    }
}

impl WindowState for DynUserState {
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
