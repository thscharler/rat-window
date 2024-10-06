use crate::window_style::{WindowDeco, WindowDecoStyle};
use crate::{Window, WindowState, WindowUserState};
use ratatui::layout::Rect;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

/// Builder for new windows.
pub struct WindowBuilder<T>
where
    T: Window,
{
    pub(crate) win: T,
    pub(crate) state: Rc<RefCell<WindowState>>,
    pub(crate) user: Rc<RefCell<dyn WindowUserState>>,
    pub(crate) deco: Option<Rc<dyn WindowDeco>>,
    pub(crate) deco_style: Option<Rc<dyn WindowDecoStyle>>,
}

impl<T> Debug for WindowBuilder<T>
where
    T: Window + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowBuilder")
            .field("win", &self.win)
            .field("state", &self.state)
            .field("user", &"..dyn..")
            .field("deco", &"..dyn..")
            .field("deco_style", &"..dyn..")
            .finish()
    }
}

impl<T> WindowBuilder<T>
where
    T: Window,
{
    pub fn new(win: T) -> Self {
        Self {
            win,
            state: Default::default(),
            user: Rc::new(RefCell::new(())),
            deco: None,
            deco_style: None,
        }
    }

    pub fn area(self, area: Rect) -> Self {
        self.state.borrow_mut().area = area;
        self
    }

    pub fn title(self, title: impl Into<String>) -> Self {
        self.state.borrow_mut().title = title.into();
        self
    }

    pub fn modal(self, modal: bool) -> Self {
        self.state.borrow_mut().modal = modal;
        self
    }

    pub fn closeable(self, closeable: bool) -> Self {
        self.state.borrow_mut().closeable = closeable;
        self
    }

    pub fn resizable(self, resizable: bool) -> Self {
        self.state.borrow_mut().resizable = resizable;
        self
    }

    pub fn moveable(self, moveable: bool) -> Self {
        self.state.borrow_mut().moveable = moveable;
        self
    }

    pub fn user_state(mut self, state: impl WindowUserState) -> Self {
        assert_eq!(self.win.state_id(), state.type_id());
        self.user = Rc::new(RefCell::new(state));
        self
    }

    pub fn deco(
        mut self,
        deco: impl WindowDeco + 'static,
        style: impl WindowDecoStyle + 'static,
    ) -> Self {
        assert_eq!(deco.style_id(), style.type_id());
        self.deco = Some(Rc::new(deco));
        self.deco_style = Some(Rc::new(style));
        self
    }
}
