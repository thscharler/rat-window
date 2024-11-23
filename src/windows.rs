use crate::window_manager::{WindowManager, WindowManagerState};
use crate::{DecoOne, WinState, WinWidget};
use rat_focus::{FocusFlag, HasFocus, Navigation};
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
pub struct Windows<'a, T, S, M = DecoOne>
where
    T: ?Sized,
    S: ?Sized,
    M: WindowManager,
{
    ///
    /// Offset of the rendered part of the [Windows] widget.
    ///
    /// This usually is a fixed offset that allows windows to
    /// be only partially rendered.
    offset: Position,

    ///
    /// Window manager.
    ///
    manager: M,

    _phantom: PhantomData<(&'a T, &'a S)>,
}

#[derive(Debug)]
pub struct WindowsState<T, S, M = DecoOne>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    /// Window manager.
    pub manager_state: RefCell<M::State>,

    /// Handles
    max_handle: Cell<usize>,
    /// The windows themselves.
    windows: RefCell<HashMap<WinHandle, Rc<RefCell<T>>>>,
    window_states: RefCell<HashMap<WinHandle, Rc<RefCell<S>>>>,
    /// Window closed during some operation.
    closed_windows: RefCell<HashSet<WinHandle>>,
}

impl<'a, T: ?Sized, S: ?Sized, M: WindowManager> Windows<'a, T, S, M> {
    /// New windows
    pub fn new(manager: M) -> Self {
        Self {
            offset: Default::default(),
            manager,
            _phantom: Default::default(),
        }
    }

    /// Set an offset for rendering the windows.
    /// With this offset it's possible to move windows partially
    /// outside the windows area to the left and top.
    ///
    /// The offset given defines the top-left corner of the Windows widget.
    ///
    /// This uses __window__ coordinates.
    pub fn offset(mut self, offset: Position) -> Self {
        self.offset = offset;
        self
    }
}

impl<'a, T, S, M> StatefulWidget for Windows<'a, T, S, M>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    type State = WindowsState<T, S, M>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut manager_state = state.manager_state.borrow_mut();
        let mut windows = state.windows.borrow_mut();
        let mut window_states = state.window_states.borrow_mut();

        manager_state.set_offset(self.offset);
        manager_state.set_area(area);

        for handle in manager_state.windows().iter().copied() {
            let window = windows.get(&handle).expect("window");
            let window_state = window_states.get_mut(&handle).expect("window");

            let mut window = window.borrow();
            let mut window_state = window_state.borrow_mut();

            self.manager
                .render_init_window(handle, window_state.get_flags(), &mut manager_state);

            let (widget_area, mut tmp_buf) =
                self.manager.render_init_buffer(handle, &mut manager_state);

            // window content
            window.render_ref(widget_area, &mut tmp_buf, window_state.as_dyn());

            // window decorations
            self.manager
                .render_window_frame(handle, &mut tmp_buf, &mut manager_state);

            // copy
            self.manager
                .render_copy_buffer(&mut tmp_buf, area, buf, &mut manager_state);

            // keep allocation
            self.manager.render_free_buffer(tmp_buf, &mut manager_state);
        }
    }
}

impl<T, S, M> HasFocus for WindowsState<T, S, M>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    fn focus(&self) -> FocusFlag {
        self.manager_state.borrow().focus()
    }

    fn area(&self) -> Rect {
        self.manager_state.borrow_mut().area()
    }

    fn navigable(&self) -> Navigation {
        Navigation::None
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    /// New state.
    pub fn new(window_manager_state: M::State) -> Self {
        Self {
            manager_state: RefCell::new(window_manager_state),
            max_handle: Default::default(),
            windows: Default::default(),
            window_states: Default::default(),
            closed_windows: Default::default(),
        }
    }

    /// Current offset for windows.
    pub fn offset(&self) -> Position {
        self.manager_state.borrow().offset()
    }

    /// Area of the given window.
    pub fn window_area(&self, handle: WinHandle) -> Rect {
        self.manager_state.borrow().window_area(handle)
    }

    /// Set the area of a window.
    pub fn set_window_area(&self, handle: WinHandle, area: Rect) {
        self.manager_state
            .borrow_mut()
            .set_window_area(handle, area);
        self.manager_state
            .borrow_mut()
            .set_window_base_area(handle, area);
    }

    /// This window has the focus?
    pub fn is_focused_window(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow().is_focused_window(handle)
    }

    /// Return the focused window handle.
    pub fn focused_window(&self) -> Option<WinHandle> {
        self.manager_state.borrow().focused_window()
    }

    /// Set the focused window.
    pub fn focus_window(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow_mut().focus_window(handle)
    }

    /// List of all windows in rendering order.
    pub fn windows(&self) -> Vec<WinHandle> {
        self.manager_state.borrow().windows().into()
    }

    /// Window at the given __screen__ coordinates.
    pub fn window_at(&self, pos: Position) -> Option<WinHandle> {
        self.manager_state.borrow().window_at(pos)
    }

    /// Open a new window.
    pub fn open_window(&self, window: (Rc<RefCell<T>>, Rc<RefCell<S>>), area: Rect) -> WinHandle {
        let handle = self.new_handle();

        window.1.borrow_mut().set_handle(handle);

        self.manager_state.borrow_mut().insert_window(handle);
        self.manager_state
            .borrow_mut()
            .set_window_area(handle, area);
        self.manager_state
            .borrow_mut()
            .set_window_base_area(handle, area);
        self.windows.borrow_mut().insert(handle, window.0);
        self.window_states.borrow_mut().insert(handle, window.1);

        handle
    }

    /// Close a window.
    pub fn close_window(&self, handle: WinHandle) -> bool {
        if self.windows.borrow_mut().remove(&handle).is_none() {
            // temporarily removed from the window list.
            self.closed_windows.borrow_mut().insert(handle);
        }
        self.manager_state.borrow_mut().remove_window(handle);
        true
    }

    /// Move a window to front.
    pub fn window_to_front(&self, handle: WinHandle) -> bool {
        self.manager_state.borrow_mut().window_to_front(handle)
    }

    /// Get the window for the given handle.
    pub fn window(&self, handle: WinHandle) -> (Rc<RefCell<T>>, Rc<RefCell<S>>) {
        (
            self.windows.borrow().get(&handle).expect("window").clone(),
            self.window_states
                .borrow()
                .get(&handle)
                .expect("window")
                .clone(),
        )
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    #[inline]
    fn new_handle(&self) -> WinHandle {
        self.max_handle.set(self.max_handle.get() + 1);
        WinHandle(self.max_handle.get())
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: WinWidget + ?Sized + 'static,
    S: WinState + ?Sized + 'static,
    M: WindowManager,
{
    /// Run an operation for a &mut Window
    ///
    /// Extracts the window for the duration and restores it
    /// afterwards.
    ///
    /// You can remove the window during this operation.
    /// You can add new windows during this operation.
    /// Everything else is a breeze anyway.
    ///
    pub fn run_for_window<R>(&self, handle: WinHandle, f: &mut dyn FnMut(&mut S) -> R) -> R {
        let window = self.windows.borrow_mut().remove(&handle).expect("window");
        let window_state = self
            .window_states
            .borrow_mut()
            .remove(&handle)
            .expect("window");

        // todo: make this panic safe
        let r = f(&mut window_state.borrow_mut());

        // not removed by the call to f()?
        if !self.closed_windows.borrow_mut().remove(&handle) {
            self.windows.borrow_mut().insert(handle, window);
            self.window_states.borrow_mut().insert(handle, window_state);
        }

        r
    }
}
