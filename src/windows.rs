use crate::deco_one::{DecoOne, DecoOneState};
use crate::WinState;
use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::StatefulWidget;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::rc::Rc;

///
/// Handle for a window.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WinHandle(usize);

#[derive(Debug)]
pub struct Windows<T: ?Sized> {
    ///
    /// Offset of the rendered part of the [Windows] widget.
    ///
    /// This usually is a fixed offset that allows windows to
    /// be only partially rendered.
    offset: Position,

    ///
    /// Window manager.
    ///
    manager: DecoOne,

    _phantom: PhantomData<T>,
}

#[derive(Debug)]
pub struct WindowsState<T>
where
    T: WinState + ?Sized + 'static,
{
    /// Area used by the widget.
    pub area: Rect,

    /// Window manager.
    pub manager_state: RefCell<DecoOneState>,

    /// Handles
    max_handle: Cell<usize>,
    /// The windows themselves.
    windows: RefCell<HashMap<WinHandle, Rc<RefCell<T>>>>,
    /// Window closed during some operation.
    closed_windows: RefCell<HashSet<WinHandle>>,
}

impl<T: ?Sized> Windows<T> {
    pub fn new(manager: DecoOne) -> Self {
        Self {
            offset: Default::default(),
            manager,
            _phantom: Default::default(),
        }
    }

    pub fn offset(mut self, offset: Position) -> Self {
        self.offset = offset;
        self
    }
}

impl<T> StatefulWidget for Windows<T>
where
    T: WinState + ?Sized + 'static,
{
    type State = WindowsState<T>;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut manager_state = state.manager_state.borrow_mut();
        let mut windows = state.windows.borrow_mut();

        state.area = area;
        manager_state.set_offset(self.offset);
        manager_state.set_area(area);

        for handle in manager_state.windows() {
            let window_state = windows.get_mut(&handle).expect("window");
            let mut window_state = window_state.borrow_mut();

            self.manager
                .prepare_window(handle, window_state.get_flags(), &mut manager_state);

            let mut tmp = self.manager.get_buffer(handle, &mut manager_state);
            let window_area = manager_state.window_widget_area(handle);

            // window content
            let window_widget = window_state.get_widget();
            window_widget.render_ref(window_area, &mut tmp, window_state.as_dyn());

            // window decorations
            self.manager
                .render_window(handle, &mut tmp, &mut manager_state);

            // copy
            self.manager
                .shift_clip_copy(&mut tmp, area, buf, &mut manager_state);

            // keep allocation
            self.manager.set_buffer(tmp, &mut manager_state);
        }
    }
}

impl<T> WindowsState<T>
where
    T: WinState + ?Sized + 'static,
{
    pub fn new() -> Self {
        Self {
            area: Default::default(),
            manager_state: RefCell::new(DecoOneState::default()),
            max_handle: Default::default(),
            windows: Default::default(),
            closed_windows: Default::default(),
        }
    }

    pub fn offset(&self) -> Position {
        self.manager_state.borrow().offset()
    }

    pub fn window_area(&self, handle: WinHandle) -> Rect {
        self.manager_state.borrow().window_area(handle)
    }

    pub fn set_window_area(&self, handle: WinHandle, area: Rect) {
        self.manager_state
            .borrow_mut()
            .set_window_area(handle, area);
        self.manager_state.borrow_mut().set_base_size(handle, area);
    }

    pub fn is_window_focused(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow().is_window_focused(handle)
    }

    pub fn focused_window(&self) -> Option<WinHandle> {
        self.manager_state.borrow().focused_window()
    }

    pub fn set_focused_window(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow_mut().set_focused_window(handle)
    }

    pub fn windows(&self) -> Vec<WinHandle> {
        self.manager_state.borrow().windows()
    }

    /// Window at the given __screen__ coordinates.
    pub fn window_at(&self, pos: Position) -> Option<WinHandle> {
        let manager_state = self.manager_state.borrow();
        let Some(pos) = manager_state.screen_to_win(pos) else {
            return None;
        };
        manager_state.window_at(pos)
    }

    pub fn open_window(&self, window: Rc<RefCell<T>>, area: Rect) -> WinHandle {
        let handle = self.new_handle();

        window.borrow_mut().set_handle(handle);

        self.manager_state.borrow_mut().insert(handle);
        self.manager_state
            .borrow_mut()
            .set_window_area(handle, area);
        self.manager_state.borrow_mut().set_base_size(handle, area);
        self.windows.borrow_mut().insert(handle, window);

        handle
    }

    pub fn close_window(&self, handle: WinHandle) -> bool {
        if self.windows.borrow_mut().remove(&handle).is_none() {
            // temporarily removed from the window list.
            self.closed_windows.borrow_mut().insert(handle);
        }
        self.manager_state.borrow_mut().remove(handle);
        true
    }

    pub fn window_to_front(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow_mut().window_to_front(handle)
    }

    pub fn window(&self, handle: WinHandle) -> Rc<RefCell<T>> {
        self.windows.borrow().get(&handle).expect("window").clone()
    }
}

impl<T> WindowsState<T>
where
    T: WinState + ?Sized + 'static,
{
    #[inline]
    fn new_handle(&self) -> WinHandle {
        self.max_handle.set(self.max_handle.get() + 1);
        WinHandle(self.max_handle.get())
    }
}

impl<T> WindowsState<T>
where
    T: WinState + ?Sized + 'static,
{
    ///
    /// Convert the mouse-event to window coordinates, if possible.
    ///
    pub fn relocate_mouse_event(&self, m: &MouseEvent) -> Option<MouseEvent> {
        if let Some(pos) = self
            .manager_state
            .borrow()
            .screen_to_win(Position::new(m.column, m.row))
        {
            let mut mm = m.clone();
            mm.column = pos.x;
            mm.row = pos.y;
            Some(mm)
        } else {
            None
        }
    }

    /// Run an operation for a &mut Window
    ///
    /// Extracts the window for the duration and restores it
    /// afterwards.
    ///
    /// You can remove the window during this operation.
    /// You can add new windows during this operation.
    /// Everything else is a breeze anyway.
    ///
    pub fn run_for_mut<R>(
        &mut self,
        handle: WinHandle,
        f: &mut dyn FnMut(&mut WindowsState<T>, &mut T) -> R,
    ) -> R {
        let window = self.windows.borrow_mut().remove(&handle).expect("window");

        // todo: make this panic safe
        let r = f(self, &mut window.borrow_mut());

        // not removed by the call to f()?
        if !self.closed_windows.borrow_mut().remove(&handle) {
            self.windows.borrow_mut().insert(handle, window);
        }

        r
    }
}
