use rat_focus::{FocusFlag, HasFocusFlag};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};
use std::fmt::Debug;

/// Trait for a window.
pub trait Window: StatefulWidgetRef<State = WindowState> + Any {}

#[derive(Debug)]
pub struct WindowState {
    ///  Window area, available after render.
    /// __read only__
    pub area: Rect,
    /// Window inner area, available after render.
    /// __read only__
    pub inner: Rect,
    ///  Window active areas, available after render.
    /// __read only__
    pub area_close: Rect,
    pub area_move: Rect,
    pub area_resize_top_left: Rect,
    pub area_resize_top: Rect,
    pub area_resize_top_right: Rect,
    pub area_resize_right: Rect,
    pub area_resize_bottom_right: Rect,
    pub area_resize_bottom: Rect,
    pub area_resize_bottom_left: Rect,
    pub area_resize_left: Rect,

    /// Window title area, available after render.
    /// __read only__
    pub area_title: Rect,

    /// Window title.
    /// __read+write__
    pub title: String,
    /// Window modality.
    /// __read+write__
    pub modal: bool,
    /// Window is closeable.
    /// __read+write__
    pub closeable: bool,
    /// Window is resizable.
    /// __read+write__
    pub resizable: bool,
    /// Window is moveable.
    /// __read+write__
    pub moveable: bool,
    /// Window focus.
    /// __read+write__
    pub focus: FocusFlag,
}

impl dyn Window {
    /// down cast Any style.
    pub fn downcast_ref<R: 'static>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn Window = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }
}

impl Window for Box<dyn Window + 'static> {}

impl StatefulWidgetRef for Box<dyn Window + 'static> {
    type State = WindowState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.as_ref().render_ref(area, buf, state);
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            inner: Default::default(),
            area_close: Default::default(),
            area_move: Default::default(),
            area_resize_top_left: Default::default(),
            area_resize_top: Default::default(),
            area_resize_top_right: Default::default(),
            area_resize_right: Default::default(),
            area_resize_bottom_right: Default::default(),
            area_resize_bottom: Default::default(),
            area_resize_bottom_left: Default::default(),
            area_resize_left: Default::default(),
            area_title: Default::default(),
            title: "".to_string(),
            modal: false,
            closeable: true,
            resizable: true,
            moveable: true,
            focus: Default::default(),
        }
    }
}

impl HasFocusFlag for WindowState {
    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl WindowState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    pub fn resizeable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn moveable(mut self, moveable: bool) -> Self {
        self.moveable = moveable;
        self
    }
}
