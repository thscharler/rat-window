use crate::window_deco::WindowDeco;
use crate::{Window, WindowState};
use ratatui::layout::Rect;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

/// Builder for new windows.
pub struct WindowBuilder<T, U>
where
    T: Window<State = U>,
    U: WindowState,
{
    pub(crate) win: T,
    pub(crate) user: U,
    pub(crate) deco: Option<Rc<dyn WindowDeco>>,
}

impl<T, U> Debug for WindowBuilder<T, U>
where
    T: Window<State = U> + Debug,
    U: WindowState + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowBuilder")
            .field("win", &self.win)
            .field("user", &self.user)
            .field("deco", &"..dyn..")
            .finish()
    }
}

impl<T, U> WindowBuilder<T, U>
where
    T: Window<State = U>,
    U: WindowState,
{
    pub fn new(win: T, user: U) -> Self {
        if win.state_id() != user.boxed_type_id() {
            panic!("state not matching window widget");
        }
        Self {
            win,
            user,
            deco: None,
        }
    }

    pub fn area(mut self, area: Rect) -> Self {
        self.user.window_mut().area = area;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.user.window_mut().title = title.into();
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.user.window_mut().modal = modal;
        self
    }

    pub fn closeable(mut self, closeable: bool) -> Self {
        self.user.window_mut().closeable = closeable;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.user.window_mut().resizable = resizable;
        self
    }

    pub fn moveable(mut self, moveable: bool) -> Self {
        self.user.window_mut().moveable = moveable;
        self
    }

    pub fn deco(mut self, deco: impl WindowDeco + 'static) -> Self {
        self.deco = Some(Rc::new(deco));
        self
    }
}
