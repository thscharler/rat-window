use crate::window_deco::WindowDeco;
use crate::{Window, WindowState, WindowUserState};
use ratatui::layout::Rect;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

/// Builder for new windows.
pub struct WindowBuilder<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    pub(crate) win: T,
    pub(crate) state: WindowState,
    pub(crate) user: U,
    pub(crate) deco: Option<Rc<dyn WindowDeco>>,
}

impl<T, U> Debug for WindowBuilder<T, U>
where
    T: Window<U> + Debug,
    U: WindowUserState + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowBuilder")
            .field("win", &self.win)
            .field("state", &self.state)
            .field("deco", &"..dyn..")
            .finish()
    }
}

impl<T, U> WindowBuilder<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    pub fn new(win: T, user: U) -> Self {
        Self {
            win,
            user,
            state: Default::default(),
            deco: None,
        }
    }

    pub fn area(mut self, area: Rect) -> Self {
        self.state.area = area;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.state.title = title.into();
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.state.modal = modal;
        self
    }

    pub fn closeable(mut self, closeable: bool) -> Self {
        self.state.closeable = closeable;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.state.resizable = resizable;
        self
    }

    pub fn moveable(mut self, moveable: bool) -> Self {
        self.state.moveable = moveable;
        self
    }

    pub fn deco(mut self, deco: impl WindowDeco + 'static) -> Self {
        self.deco = Some(Rc::new(deco));
        self
    }
}
