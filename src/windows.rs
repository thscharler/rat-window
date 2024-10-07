use crate::deco::One;
use crate::utils::copy_buffer;
use crate::window_deco::{WindowDeco, WindowDecoStyle};
use crate::{Error, Window, WindowBuilder, WindowState, WindowUserState};
use bimap::BiMap;
use rat_event::{ct_event, flow, HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{ContainerFlag, FocusBuilder, HasFocus, HasFocusFlag};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::StatefulWidget;
use std::any::Any;
use std::cmp::max;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::rc::Rc;

/// Handle returned for an added window. Used as a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(usize);

/// Window handler
pub struct Windows<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    _phantom: PhantomData<(T, U)>,
    // deco styles per deco
    deco_style: Vec<Box<dyn WindowDecoStyle + 'static>>,
}

pub struct WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    /// last rendered area for windowing.
    /// __read-only__
    pub area: Rect,

    /// Offset of the displayed area of the window pane.
    ///
    /// The window pane extends by this offset beyond the currently
    /// visible area, and windows are limited to this space.
    /// This way windows can be moved partially outside the pane
    /// without negative coords (which don't exist).
    ///
    /// For the right/bottom border this is a somewhat soft border.
    /// You can manually place windows beyond, and resizing the
    /// terminal will also not affect the window positions.
    ///
    /// __read+write___
    pub zero_offset: Position,

    /// default decorations
    /// __read+write__
    pub default_deco: Rc<dyn WindowDeco>,

    /// is the complete windows widget focused?
    pub focus: ContainerFlag,

    // max handle
    max_id: usize,
    // window handles
    win_handle: BiMap<WindowHandle, usize>,
    // window widget
    win: Vec<WinStruct<T, U>>,

    // mouse stuff
    mouse: WinMouseFlags,
}

struct WinStruct<T, U> {
    win: T,
    // overall window state
    state: WindowState,
    // user state
    user: U,
    // frame decoration
    deco: Rc<dyn WindowDeco>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinMouseArea {
    Close,
    Move,
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Debug, Clone, Copy)]
struct WinMouseFlags {
    drag_base: Option<Rect>,
    drag_zero: Option<Position>,
    drag_win: Option<usize>,
    drag_area: Option<WinMouseArea>,
}

impl<T, U> Debug for Windows<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Windows")
            .field("deco_style", &"..dyn..")
            .finish()
    }
}

impl<T, U> Default for Windows<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
            deco_style: vec![],
        }
    }
}

impl<T, U> Windows<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deco(mut self, deco_style: impl WindowDecoStyle) -> Self {
        // todo: check for duplicates?
        self.deco_style.push(Box::new(deco_style));
        self
    }
}

impl<T, U> StatefulWidget for Windows<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    type State = WindowsState<T, U>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;

        // necessary buffer area. only need enough for the windows.
        let mut tmp_area: Option<Rect> = None;
        let mut it = state.win.iter();
        loop {
            let Some(win) = it.next() else {
                break;
            };

            if let Some(tmp_area) = tmp_area.as_mut() {
                *tmp_area = tmp_area.union(win.state.area);
            } else {
                tmp_area = Some(win.state.area);
            }
        }
        let tmp_area = tmp_area.unwrap_or_default();

        // buffer is constructed with windows coordinates
        let mut tmp = Buffer::empty(Rect::new(
            tmp_area.x,
            tmp_area.y,
            tmp_area.width,
            tmp_area.height,
        ));
        let tmp = &mut tmp;

        for WinStruct {
            win,
            state: win_state,
            user: win_user,
            deco: win_deco,
        } in state.win.iter_mut()
        {
            // Find window styles.
            let mut win_deco_style = None;
            for style in self.deco_style.iter() {
                if win_deco.style_id() == style.as_ref().type_id() {
                    win_deco_style = Some(style.as_ref());
                    break;
                }
            }

            // decorations
            let area = win_state.area;
            win_deco.render_ref(area, tmp, win_deco_style, win_state, win_user);

            // content
            let inner = win_state.inner;
            win.render_ref(inner, tmp, win_state, win_user);
        }

        copy_buffer(tmp, state.zero_offset, area, buf);
    }
}

impl<T, U> Debug for WindowsState<T, U>
where
    T: Window<U>,
    T: Debug,
    U: WindowUserState,
    U: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsState")
            .field("area", &self.area)
            .field("zero_offset", &self.zero_offset)
            .field("max_id", &self.max_id)
            .field("win_handle", &self.win_handle)
            .field("win", &self.win)
            .field("mouse", &self.mouse)
            .finish()
    }
}

impl<T, U> Default for WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    fn default() -> Self {
        Self {
            area: Default::default(),
            zero_offset: Default::default(),
            default_deco: Rc::new(One),
            focus: Default::default(),
            max_id: 0,
            win_handle: Default::default(),
            win: vec![],
            mouse: Default::default(),
        }
    }
}

impl<T, U> HasFocus for WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
    U: HasFocus,
{
    fn build(&self, builder: &mut FocusBuilder) {
        for WinStruct { state, user, .. } in self.win.iter() {
            if state.focus.is_focused() {
                builder.container(user);
            }
        }
    }

    fn container(&self) -> Option<ContainerFlag> {
        Some(self.focus.clone())
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl<T, U> HandleEvent<crossterm::event::Event, Regular, Outcome> for WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
    U: HasFocus,
    U: HandleEvent<crossterm::event::Event, Regular, Outcome>,
    U: HandleEvent<crossterm::event::Event, MouseOnly, Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        flow!(self.handle(event, MouseOnly));

        flow!({
            let mut r = Outcome::Continue;
            //if self.is_focused() {
            for WinStruct { state, user, .. } in self.win.iter_mut() {
                if state.focus.is_focused() {
                    let u = user.handle(event, Regular);
                    r = max(r, u);
                }
            }
            //}
            r
        });

        Outcome::Continue
    }
}

impl<T, U> HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
    U: HandleEvent<crossterm::event::Event, MouseOnly, Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        flow!(match event {
            ct_event!(mouse down Left for x,y) => {
                let mut r = Outcome::Continue;
                // focus and front window
                if let Some((h, _)) = self.to_front_at(Position::new(*x, *y)) {
                    r = self.focus_window(h).into();
                }

                // Test for some draggable area.
                self.at_hit(Position::new(*x, *y), |windows, pos, _handle, idx_win| {
                    let win = &windows.win[idx_win].state;

                    let areas = [
                        win.area_close,
                        win.area_move,
                        win.area_resize_top_left,
                        win.area_resize_top,
                        win.area_resize_top_right,
                        win.area_resize_right,
                        win.area_resize_bottom_right,
                        win.area_resize_bottom,
                        win.area_resize_bottom_left,
                        win.area_resize_left,
                    ];
                    for (idx_area, area) in areas.iter().enumerate() {
                        if area.contains(pos.into()) {
                            windows.mouse.drag_zero = Some(pos);
                            windows.mouse.drag_base = Some(win.area);
                            windows.mouse.drag_win = Some(idx_win);
                            windows.mouse.drag_area = Some(idx_area.into());
                            break;
                        }
                    }
                });

                r
            }
            ct_event!(mouse drag Left for x,y) => 'f: {
                let Some(win_idx) = self.mouse.drag_win else {
                    break 'f Outcome::Continue;
                };

                match self.mouse.drag_area {
                    None => Outcome::Continue,
                    Some(WinMouseArea::Move) => {
                        let pos = self.screen_to_window_pos(Position::new(*x, *y));
                        let zero = self.mouse.drag_zero.expect("zero");
                        let base = self.mouse.drag_base.expect("base");

                        let bounds = self.windows_area();

                        // move
                        let state = &mut self.win[win_idx].state;
                        state.area.x = (base.x + pos.x).saturating_sub(zero.x);
                        state.area.y = (base.y + pos.y).saturating_sub(zero.y);

                        // limit movement
                        if state.area.right() >= bounds.right() {
                            state.area.x = state
                                .area
                                .x
                                .saturating_sub(state.area.right() - bounds.right())
                        }
                        if state.area.bottom() >= bounds.bottom() {
                            state.area.y = state
                                .area
                                .y
                                .saturating_sub(state.area.bottom() - bounds.bottom())
                        }

                        Outcome::Changed
                    }
                    _ => Outcome::Continue,
                }
            }
            ct_event!(mouse up Left for _x, _y) | ct_event!(mouse moved for _x, _y) => {
                self.mouse.drag_zero = None;
                self.mouse.drag_base = None;
                self.mouse.drag_win = None;
                self.mouse.drag_area = None;
                Outcome::Continue
            }
            _ => Outcome::Continue,
        });

        let mut r = Outcome::Continue;
        for WinStruct { user, .. } in self.win.iter_mut() {
            let u = user.handle(event, MouseOnly);
            r = max(r, u);
        }
        r
    }
}

impl<T, U> WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Offset of the displayed area of the window pane.
    ///
    /// The window pane extends by this offset beyond the currently
    /// visible area, and windows are limited to this space.
    /// This way windows can be moved partially outside the pane
    /// without negative coords (which don't exist).
    pub fn zero_offset(mut self, x: u16, y: u16) -> Self {
        self.zero_offset = Position::new(x, y);
        self
    }

    /// Default window decorations.
    pub fn deco(mut self, deco: impl WindowDeco + 'static) -> Self {
        self.default_deco = Rc::new(deco);
        self
    }

    /// Change the deco-style for all windows.
    /// Doesn't change the default, use deco for that.
    ///
    /// Changes only windows that have the same deco/style type-id.
    pub fn change_deco(&mut self, deco: impl WindowDeco + 'static) {
        let new_deco = Rc::new(deco);

        for w in self.win.iter_mut() {
            if w.deco.type_id() == new_deco.type_id() {
                w.deco = new_deco.clone();
            }
        }
    }
}

impl<T, U> WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    /// Get the bounds for the windows coordinate system.
    /// This always starts at 0,0 and extends to
    /// zero.x+width+zero.x / zero.y+height+zero.y
    ///
    /// Windows are constrained to this area.
    ///
    /// This way windows can be moved partially outside the bounds
    /// of the windows area without falling back to negative coords
    /// (which don't exist).
    pub fn windows_area(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.zero_offset.x + self.area.width + self.zero_offset.x,
            self.zero_offset.x + self.area.height + self.zero_offset.y,
        )
    }

    /// Show a window.
    ///
    /// The builder parameter looks quit impressive, but you want
    /// to use WindowBuilder for that anyway.
    pub fn show(&mut self, builder: WindowBuilder<T, U>) -> WindowHandle {
        let handle = self.new_handle();
        let idx = self.win.len();
        self.win_handle
            .insert_no_overwrite(handle, idx)
            .expect("no duplicate");

        let mut st = WinStruct {
            win: builder.win,
            state: builder.state,
            user: builder.user,
            deco: builder.deco.unwrap_or(self.default_deco.clone()),
        };

        // some sensible defaults...
        {
            if st.state.area.is_empty() {
                st.state.area = Rect::new(
                    self.zero_offset.x,
                    self.zero_offset.y,
                    self.area.width,
                    self.area.height,
                );
            }
        }

        self.win.push(st);

        handle
    }

    /// Move window at position to the front.
    ///
    /// This takes screen coordinates.
    ///
    /// __Panic__
    ///
    /// Panics if pos is not in bounds of the windows area.
    pub fn focus_window_at(&mut self, pos: Position) -> Option<(WindowHandle, bool)> {
        self.at_hit(pos, |w, _, handle, _| {
            // focus
            w.focus_window(handle)
        })
    }

    /// Focus the given window.
    /// Doesn't move the window to the front. Use to_front... for that.
    pub fn focus_window(&mut self, h: WindowHandle) -> bool {
        self.try_focus_window(h).expect("valid handle")
    }

    /// Focus the given window.
    /// Doesn't move the window to the front. Use to_front... for that.
    ///
    /// Doesn't focus the first widget in the window.
    /// Use Focus::first_container(windows)
    pub fn try_focus_window(&mut self, h: WindowHandle) -> Result<bool, Error> {
        let idx_win = self.try_handle_idx(h)?;

        let old_focus = self.win[idx_win].state.focus.is_focused();

        for (idx, win) in self.win.iter().enumerate() {
            if idx_win == idx {
                win.state.focus.set(true);
            } else {
                win.state.focus.set(false);
            }
        }

        Ok(!old_focus)
    }

    /// Move window at position to the front.
    ///
    /// This takes screen coordinates.
    ///
    /// __Panic__
    ///
    /// Panics if pos is not in bounds of the windows area.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_front_at(&mut self, pos: Position) -> Option<(WindowHandle, bool)> {
        self.at_hit(pos, |w, _, handle, _| {
            // to front
            w.to_front(handle)
        })
    }

    /// Move window to the front.
    ///
    #[allow(clippy::wrong_self_convention)]
    pub fn to_front(&mut self, h: WindowHandle) -> bool {
        self.try_to_front(h).expect("valid handle")
    }

    /// Move window to the front.
    pub fn try_to_front(&mut self, h: WindowHandle) -> Result<bool, Error> {
        let max_idx = self.win.len() - 1;

        // extract data
        let (_h, idx_win) = self
            .win_handle
            .remove_by_left(&h)
            .ok_or(Error::InvalidHandle)?;

        let win = self.win.remove(idx_win);

        // correct handle mappings, shift left
        for cor in idx_win + 1..=max_idx {
            let (h, _) = self.win_handle.remove_by_right(&cor).expect("valid win");
            self.win_handle
                .insert_no_overwrite(h, cor - 1)
                .expect("no duplicates")
        }

        // reinstate
        self.win.push(win);

        self.win_handle
            .insert_no_overwrite(h, max_idx)
            .expect("no duplicates");

        // todo: is this necessary and correct?
        self.mouse.clear();

        Ok(idx_win != max_idx)
    }
}

impl<T, U> WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    pub fn windows(&self) -> impl Iterator<Item = WindowHandle> + '_ {
        self.win_handle.left_values().copied()
    }

    pub fn window(&self, handle: WindowHandle) -> &T {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        &self.win[idx].win
    }

    pub fn try_window(&self, handle: WindowHandle) -> Result<&T, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(&self.win[idx].win)
    }

    pub fn window_state(&self, handle: WindowHandle) -> &WindowState {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        &self.win[idx].state
    }

    pub fn try_window_state(&self, handle: WindowHandle) -> Result<&WindowState, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(&self.win[idx].state)
    }

    pub fn user_state(&self, handle: WindowHandle) -> &U {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        &self.win[idx].user
    }

    pub fn try_user_state(&self, handle: WindowHandle) -> Result<&U, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(&self.win[idx].user)
    }

    pub fn frame(&self, handle: WindowHandle) -> Rc<dyn WindowDeco> {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        self.win[idx].deco.clone()
    }

    pub fn try_frame(&self, handle: WindowHandle) -> Result<Rc<dyn WindowDeco>, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(self.win[idx].deco.clone())
    }
}

impl<T, U> WindowsState<T, U>
where
    T: Window<U>,
    U: WindowUserState,
{
    // construct handle
    fn new_handle(&mut self) -> WindowHandle {
        self.max_id += 1;
        WindowHandle(self.max_id)
    }

    // idx for handle
    fn try_handle_idx(&self, handle: WindowHandle) -> Result<usize, Error> {
        self.win_handle
            .get_by_left(&handle)
            .copied()
            .ok_or(Error::InvalidHandle)
    }

    // idx for handle
    #[allow(dead_code)]
    fn handle_idx(&self, handle: WindowHandle) -> usize {
        self.win_handle
            .get_by_left(&handle)
            .copied()
            .expect("valid handle")
    }

    // handle for idx
    fn idx_handle(&self, idx: usize) -> WindowHandle {
        self.win_handle
            .get_by_right(&idx)
            .copied()
            .expect("valid idx")
    }

    // finds a hit and executes some action
    fn at_hit<R>(
        &mut self,
        pos: Position,
        f: impl FnOnce(&mut WindowsState<T, U>, Position, WindowHandle, usize) -> R,
    ) -> Option<(WindowHandle, R)> {
        let pos = self.screen_to_window_pos(pos);

        // focus and front window
        let mut it = self.win.iter().enumerate().rev();
        loop {
            let Some((idx_win, win)) = it.next() else {
                break;
            };
            if win.state.area.contains(pos) {
                let handle = self.idx_handle(idx_win);
                let r = f(self, pos, handle, idx_win);
                return Some((handle, r));
            }
        }
        None
    }

    // transformation from terminal-space to windows-space
    #[allow(dead_code)]
    pub fn screen_to_window_rect(&self, rect: Rect) -> Rect {
        let x = (rect.x - self.area.x) + self.zero_offset.x;
        let y = (rect.y - self.area.y) + self.zero_offset.y;
        Rect::new(x, y, rect.width, rect.height)
    }

    // transformation from terminal-space to windows-space
    pub fn screen_to_window_pos(&self, pos: Position) -> Position {
        let x = (pos.x - self.area.x) + self.zero_offset.x;
        let y = (pos.y - self.area.y) + self.zero_offset.y;
        Position::new(x, y)
    }
}

impl<T, U> Debug for WinStruct<T, U>
where
    T: Window<U> + Debug,
    U: WindowUserState + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinStruct")
            .field("win", &self.win)
            .field("state", &self.state)
            .field("frame", &"..dyn..")
            .field("frame_style", &"..dyn..")
            .finish()
    }
}

impl From<usize> for WinMouseArea {
    fn from(value: usize) -> Self {
        match value {
            0 => WinMouseArea::Close,
            1 => WinMouseArea::Move,
            2 => WinMouseArea::TopLeft,
            3 => WinMouseArea::Top,
            4 => WinMouseArea::TopRight,
            5 => WinMouseArea::Right,
            6 => WinMouseArea::BottomRight,
            7 => WinMouseArea::Bottom,
            8 => WinMouseArea::BottomLeft,
            9 => WinMouseArea::Left,
            _ => unreachable!(),
        }
    }
}

impl Default for WinMouseFlags {
    fn default() -> Self {
        Self {
            drag_base: None,
            drag_zero: None,
            drag_win: None,
            drag_area: None,
        }
    }
}

impl WinMouseFlags {
    pub(crate) fn clear(&mut self) {
        self.drag_zero = None;
        self.drag_base = None;
        self.drag_area = None;
        self.drag_win = None;
    }
}
