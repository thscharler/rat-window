use crate::window_manager::{WindowManager, WindowManagerState};
use crate::{DecoOne, WinFlags};
use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer, FocusFlag, HasFocus, Navigation};
use ratatui::layout::{Position, Rect};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
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

impl<T, S, M> Debug for WindowsState<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager + Debug,
    M::State: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsState")
            .field("rc", self.rc.as_ref())
            .finish()
    }
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

impl<T, S, M> Debug for WindowsStateRc<T, S, M>
where
    T: ?Sized + 'static,
    S: ?Sized + 'static,
    M: WindowManager + Debug,
    M::State: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let windows = self.windows.borrow().keys().copied().collect::<Vec<_>>();

        f.debug_struct("WindowsStateRc")
            .field("manager", &self.manager)
            .field("max_handle", &self.max_handle)
            .field("windows", &windows)
            .field("closed_windows", &self.closed_windows)
            .finish()
    }
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
    ///
    /// Sets both the window_area and the base_area of the window.
    /// This calls [self.add_offset()] to place the area relative to
    /// the visible area.
    pub fn set_window_area(&self, handle: WinHandle, area: Rect) {
        let area = self.add_offset(area);
        self.rc.manager.borrow_mut().set_window_area(handle, area);
        self.rc
            .manager
            .borrow_mut()
            .set_window_base_area(handle, area);
    }

    /// Flags for a window.
    pub fn window_flags(&self, handle: WinHandle) -> WinFlags {
        self.rc.manager.borrow().window_flags(handle)
    }

    /// Set flags for a window.
    pub fn set_window_flags(&self, handle: WinHandle, flags: WinFlags) {
        self.rc.manager.borrow_mut().set_window_flags(handle, flags);
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

    /// Add the offset to the given area.
    /// This is useful when you create new windows and don't
    /// want to have them outside the visible area anyway.
    pub fn add_offset(&self, area: Rect) -> Rect {
        self.rc.manager.borrow().add_offset(area)
    }

    /// Open a new window with defaults.
    ///
    /// You probably want to call
    /// - [self.set_window_area] to set an actual area for the window.
    /// - [self.set_window_flags] to change the appearance and behaviour.
    ///
    pub fn open_window(&self, window: (Rc<RefCell<T>>, Rc<RefCell<S>>)) -> WinHandle {
        let handle = self.new_handle();

        self.rc.manager.borrow_mut().insert_window(handle);
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
    pub fn window(&self, handle: WinHandle) -> Rc<RefCell<T>> {
        self.rc
            .windows
            .borrow()
            .get(&handle)
            .expect("window")
            .clone()
    }

    /// Get the window for the given handle.
    pub fn window_state(&self, handle: WinHandle) -> Rc<RefCell<S>> {
        self.rc
            .window_states
            .borrow()
            .get(&handle)
            .expect("window")
            .clone()
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
