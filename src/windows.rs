use crate::win_base::WinBaseState;
use crate::window_manager::{WindowManager, WindowManagerState};
use crate::DecoOne;
use rat_focus::{FocusFlag, HasFocus, Navigation};
use ratatui::layout::{Position, Rect};
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
pub struct Windows<'a, S, M = DecoOne>
where
    M: WindowManager,
    S: ?Sized,
{
    ///
    /// Offset of the rendered part of the [Windows] widget.
    ///
    /// This usually is a fixed offset that allows windows to
    /// be only partially rendered.
    pub offset: Position,

    ///
    /// Window manager.
    ///
    pub manager: M,

    ///
    pub _phantom: PhantomData<&'a S>,
}

pub struct WindowsState<T, S, M = DecoOne>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    pub rc: Rc<WindowsStateRc<T, S, M>>,
}

impl<T, S, M> Clone for WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    fn clone(&self) -> Self {
        Self {
            rc: self.rc.clone(),
        }
    }
}

pub struct WindowsStateRc<T, S, M = DecoOne>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    /// Window manager.
    pub manager: RefCell<M::State>,

    /// Handles
    max_handle: Cell<usize>,
    /// The windows themselves.
    windows: RefCell<HashMap<WinHandle, Rc<RefCell<T>>>>,
    window_states: RefCell<HashMap<WinHandle, Rc<RefCell<S>>>>,
    /// Window closed during some operation.
    closed_windows: RefCell<HashSet<WinHandle>>,
}

impl<'a, S: ?Sized, M: WindowManager> Windows<'a, S, M> {
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

impl<T, S, M> HasFocus for WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    fn focus(&self) -> FocusFlag {
        self.rc.manager.borrow().focus()
    }

    fn area(&self) -> Rect {
        self.rc.manager.borrow_mut().area()
    }

    fn navigable(&self) -> Navigation {
        Navigation::None
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    /// New state.
    pub fn new(window_manager_state: M::State) -> Self {
        Self {
            rc: Rc::new(WindowsStateRc {
                manager: RefCell::new(window_manager_state),
                max_handle: Default::default(),
                windows: Default::default(),
                window_states: Default::default(),
                closed_windows: Default::default(),
            }),
        }
    }

    /// Current offset for windows.
    pub fn offset(&self) -> Position {
        self.rc.manager.borrow().offset()
    }

    /// Area of the given window.
    pub fn window_area(&self, handle: WinHandle) -> Rect {
        self.rc.manager.borrow().window_area(handle)
    }

    /// Set the area of a window.
    pub fn set_window_area(&self, handle: WinHandle, area: Rect) {
        self.rc.manager.borrow_mut().set_window_area(handle, area);
        self.rc
            .manager
            .borrow_mut()
            .set_window_base_area(handle, area);
    }

    /// This window has the focus?
    pub fn is_focused_window(&self, handle: WinHandle) -> bool {
        self.rc.manager.borrow().is_focused_window(handle)
    }

    /// Return the focused window handle.
    pub fn focused_window(&self) -> Option<WinHandle> {
        self.rc.manager.borrow().focused_window()
    }

    /// Set the focused window.
    pub fn focus_window(&self, handle: WinHandle) -> bool {
        self.rc.manager.borrow_mut().focus_window(handle)
    }

    /// List of all windows in rendering order.
    pub fn handles(&self) -> Vec<WinHandle> {
        self.rc.manager.borrow().handles().into()
    }

    /// Window at the given __screen__ coordinates.
    pub fn window_at(&self, pos: Position) -> Option<WinHandle> {
        self.rc.manager.borrow().window_at(pos)
    }

    /// Change the widget used for the given window.
    pub fn set_window_widget(&self, handle: WinHandle, widget: Rc<RefCell<T>>) {
        self.rc.windows.borrow_mut().insert(handle, widget);
    }

    /// Change the state for the given window.
    pub fn set_window_state(&self, handle: WinHandle, state: Rc<RefCell<S>>) {
        self.rc.window_states.borrow_mut().insert(handle, state);
    }

    /// Open a new window.
    pub fn open_window(&self, window: (Rc<RefCell<T>>, Rc<RefCell<S>>), area: Rect) -> WinHandle
    where
        S: WinBaseState,
    {
        let handle = self.new_handle();

        window.1.borrow_mut().set_handle(handle);

        self.rc.manager.borrow_mut().insert_window(handle);
        self.rc.manager.borrow_mut().set_window_area(handle, area);
        self.rc
            .manager
            .borrow_mut()
            .set_window_base_area(handle, area);
        self.rc.windows.borrow_mut().insert(handle, window.0);
        self.rc.window_states.borrow_mut().insert(handle, window.1);

        handle
    }

    /// Close a window.
    pub fn close_window(&self, handle: WinHandle) -> bool {
        if self.rc.windows.borrow_mut().remove(&handle).is_none() {
            // temporarily removed from the window list.
            self.rc.closed_windows.borrow_mut().insert(handle);
        }
        self.rc.manager.borrow_mut().remove_window(handle);
        true
    }

    /// Move a window to front.
    pub fn window_to_front(&self, handle: WinHandle) -> bool {
        self.rc.manager.borrow_mut().window_to_front(handle)
    }

    /// Get the window for the given handle.
    pub fn window(&self, handle: WinHandle) -> (Rc<RefCell<T>>, Rc<RefCell<S>>) {
        (
            self.rc
                .windows
                .borrow()
                .get(&handle)
                .expect("window")
                .clone(),
            self.rc
                .window_states
                .borrow()
                .get(&handle)
                .expect("window")
                .clone(),
        )
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager,
{
    #[inline]
    fn new_handle(&self) -> WinHandle {
        self.rc.max_handle.set(self.rc.max_handle.get() + 1);
        WinHandle(self.rc.max_handle.get())
    }
}

impl<T, S, M> WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
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
    pub fn run_for_window<R>(
        &self,
        handle: WinHandle,
        f: &mut dyn FnMut(&mut T, &mut S) -> R,
    ) -> R {
        let (window, window_state) = {
            (
                self.rc
                    .windows
                    .borrow_mut()
                    .remove(&handle)
                    .expect("window"),
                self.rc
                    .window_states
                    .borrow_mut()
                    .remove(&handle)
                    .expect("window"),
            )
        };

        // todo: make this panic safe
        let r = f(&mut window.borrow_mut(), &mut window_state.borrow_mut());

        // not removed by the call to f()?
        if !self.rc.closed_windows.borrow_mut().remove(&handle) {
            self.rc.windows.borrow_mut().insert(handle, window);
            self.rc
                .window_states
                .borrow_mut()
                .insert(handle, window_state);
        } else {
            self.rc.closed_windows.borrow_mut().remove(&handle);
        }

        r
    }
}
