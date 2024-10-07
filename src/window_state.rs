use rat_focus::FocusFlag;
use ratatui::layout::Rect;

#[derive(Debug)]
pub struct WindowState {
    /// Window inner area, available after render.
    /// __read only__
    pub inner: Rect,
    ///  Window close area, available after render.
    /// __read only__
    pub area_close: Rect,
    ///  Window move area, available after render.
    /// __read only__
    pub area_move: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_top_left: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_top: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_top_right: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_right: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_bottom_right: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_bottom: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_bottom_left: Rect,
    ///  Window resize area, available after render.
    /// __read only__
    pub area_resize_left: Rect,
    /// Window title area, available after render.
    /// __read only__
    pub area_title: Rect,

    /// Window area, in windows coordinates.
    /// __read+write__
    pub area: Rect,

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
            focus: FocusFlag::named("window"),
        }
    }
}

impl WindowState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_area(mut self, area: Rect) -> Self {
        self.area = area;
        self
    }

    pub fn set_title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn set_modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn set_closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    pub fn set_resizeable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn set_moveable(mut self, moveable: bool) -> Self {
        self.moveable = moveable;
        self
    }
}
