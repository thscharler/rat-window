use rat_focus::{FocusFlag, HasFocusFlag};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidgetRef;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Trait for a window.
pub trait Window:
    StatefulWidgetRef<State = (Rc<RefCell<WindowState>>, Rc<RefCell<dyn WindowUserState>>)> + Any
{
    /// Return the type-id of a compatible WindowUserState.
    fn state_id(&self) -> TypeId;
}

pub trait WindowUserState: Any {}

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

    /// down cast Any style.
    pub fn downcast_mut<R: 'static>(&mut self) -> &mut R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *mut dyn Window = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
    }
}

impl Window for Box<dyn Window + 'static> {
    fn state_id(&self) -> TypeId {
        self.as_ref().state_id()
    }
}

impl StatefulWidgetRef for Box<dyn Window + 'static> {
    type State = (Rc<RefCell<WindowState>>, Rc<RefCell<dyn WindowUserState>>);

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.as_ref().render_ref(area, buf, state);
    }
}

impl WindowUserState for () {}

impl dyn WindowUserState {
    /// down cast Any style.
    pub fn downcast_ref<R: 'static>(&self) -> &R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WindowUserState = self;
            unsafe { &*(p as *const R) }
        } else {
            panic!("wrong type")
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: 'static>(&mut self) -> &mut R {
        if self.type_id() == TypeId::of::<R>() {
            let p: *mut dyn WindowUserState = self;
            unsafe { &mut *(p as *mut R) }
        } else {
            panic!("wrong type")
        }
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
