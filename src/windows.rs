use crate::deco::One;
use crate::deco_one::OneStyle;
use crate::utils::{copy_buffer, fill_buf_area};
use crate::window_style::{WindowFrame, WindowFrameStyle};
use crate::{Error, Window, WindowState, WindowUserState};
use bimap::BiMap;
use rat_event::{ct_event, HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::HasFocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{StatefulWidget, Style};
use ratatui::widgets::Block;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::rc::Rc;

/// Handle returned for an added window. Used as a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(usize);

/// Window handler
#[derive(Debug)]
pub struct Windows<T>
where
    T: Window,
{
    _phantom: PhantomData<T>,
}

pub struct WindowsState<T>
where
    T: Window,
{
    /// last rendered area for windowing.
    /// __read-only__
    pub area: Rect,

    /// offset of the window pane.
    /// there are no negative coordinates.
    /// this offset makes good for it.
    /// __read+write___
    pub zero_offset: Position,

    /// default decorations
    /// __read+write__
    pub default_deco: Rc<dyn WindowFrame>,
    pub default_deco_style: Rc<dyn WindowFrameStyle>,

    // max handle
    max_id: usize,
    // window handles
    win_handle: BiMap<WindowHandle, usize>,
    // window widget
    win: Vec<WinStruct<T>>,

    // mouse stuff
    mouse: WinMouseFlags,
}

struct WinStruct<T> {
    win: T,
    // overall window state
    state: Rc<RefCell<WindowState>>,
    // user data
    user_state: Rc<RefCell<dyn WindowUserState>>,
    // frame decoration
    frame: Rc<dyn WindowFrame>,
    // frame decoration styles
    frame_style: Rc<dyn WindowFrameStyle>,
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

impl<T> Default for Windows<T>
where
    T: Window,
{
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T> Windows<T>
where
    T: Window,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> StatefulWidget for Windows<T>
where
    T: Window,
{
    type State = WindowsState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;

        // necessary buffer
        let mut tmp_area: Option<Rect> = None;
        let mut it = state.win.iter();
        loop {
            let Some(win) = it.next() else {
                break;
            };

            if let Some(tmp_area) = tmp_area.as_mut() {
                *tmp_area = tmp_area.union(win.state.borrow().area);
            } else {
                tmp_area = Some(win.state.borrow().area);
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
            user_state: win_user_state,
            frame: win_frame,
            frame_style: win_frame_style,
        } in state.win.iter()
        {
            // Clear out window area
            fill_buf_area(tmp, win_state.borrow().area, " ", Style::default());

            // decorations
            let area = win_state.borrow().area;
            win_frame.render_ref(area, tmp, &mut (win_state.clone(), win_frame_style.clone()));

            // content
            let inner = win_state.borrow().inner;
            win.render_ref(inner, tmp, &mut (win_state.clone(), win_user_state.clone()));
        }

        copy_buffer(tmp, state.zero_offset, area, buf);
    }
}

impl<T> Debug for WindowsState<T>
where
    T: Window,
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsState")
            .field("area", &self.area)
            .field("zero", &self.zero_offset)
            .field("max_id", &self.max_id)
            .field("win_handle", &self.win_handle)
            .field("win", &self.win)
            .field("mouse", &self.mouse)
            .finish()
    }
}

impl<T> Default for WindowsState<T>
where
    T: Window,
{
    fn default() -> Self {
        Self {
            area: Default::default(),
            zero_offset: Default::default(),
            default_deco: Rc::new(One),
            default_deco_style: Rc::new(OneStyle {
                block: Block::bordered(),
                title_style: None,
                title_alignment: None,
                focus_style: None,
                ..Default::default()
            }),
            max_id: 0,
            win_handle: Default::default(),
            win: vec![],
            mouse: Default::default(),
        }
    }
}

impl<T> HandleEvent<crossterm::event::Event, Regular, Outcome> for WindowsState<T>
where
    T: Window,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        self.handle(event, MouseOnly)
    }
}

impl<T> HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for WindowsState<T>
where
    T: Window,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        match event {
            ct_event!(mouse down Left for x,y) => {
                let mut r = Outcome::Continue;
                // focus and front window
                if let Some((h, _)) = self.to_front_at(Position::new(*x, *y)) {
                    r = self.focus_window(h).into();
                }

                // Test for some draggable area.
                self.at_hit(Position::new(*x, *y), |windows, pos, _handle, idx_win| {
                    let win = windows.win[idx_win].state.borrow();

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

                        let mut state = self.win[win_idx].state.borrow_mut();
                        state.area.x = (base.x + pos.x).saturating_sub(zero.x);
                        state.area.y = (base.y + pos.y).saturating_sub(zero.y);

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
        }
    }
}

impl<T> WindowsState<T>
where
    T: Window,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Zero point for the internal coordinate system.
    pub fn zero(mut self, x: u16, y: u16) -> Self {
        self.zero_offset = Position::new(x, y);
        self
    }

    /// Default window decorations.
    pub fn deco(mut self, deco: impl WindowFrame + 'static) -> Self {
        self.default_deco = Rc::new(deco);
        self
    }

    /// Default window decoration styling.
    pub fn deco_style(mut self, style: impl WindowFrameStyle + 'static) -> Self {
        self.default_deco_style = Rc::new(style);
        self
    }
}

impl<T> WindowsState<T>
where
    T: Window,
{
    /// Show with bounds.
    ///
    /// Bounds are relative to the zero-point.
    pub fn show_at<U: WindowUserState>(
        &mut self,
        window: T,
        state: WindowState,
        user: U,
        bounds: Rect,
    ) -> WindowHandle {
        let handle = self.new_handle();
        let idx = self.win.len();
        self.win_handle
            .insert_no_overwrite(handle, idx)
            .expect("no duplicate");

        let mut state = state;
        state.area = bounds;

        self.win.push(WinStruct {
            win: window,
            state: Rc::new(RefCell::new(state)),
            user_state: Rc::new(RefCell::new(user)),
            frame: Rc::clone(&self.default_deco),
            frame_style: Rc::clone(&self.default_deco_style),
        });

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
    pub fn try_focus_window(&mut self, h: WindowHandle) -> Result<bool, Error> {
        let idx_win = self.try_handle_idx(h)?;

        let old_focus = self.win[idx_win].state.borrow().is_focused();

        for (idx, win) in self.win.iter().enumerate() {
            if idx_win == idx {
                win.state.borrow().focus().set(true);
            } else {
                win.state.borrow().focus().set(false);
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

impl<T> WindowsState<T>
where
    T: Window,
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

    pub fn window_state(&self, handle: WindowHandle) -> Rc<RefCell<WindowState>> {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        self.win[idx].state.clone()
    }

    pub fn try_window_state(
        &self,
        handle: WindowHandle,
    ) -> Result<Rc<RefCell<WindowState>>, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(self.win[idx].state.clone())
    }

    pub fn user_state(&self, handle: WindowHandle) -> Rc<RefCell<dyn WindowUserState>> {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        self.win[idx].user_state.clone()
    }

    pub fn try_user_state(
        &self,
        handle: WindowHandle,
    ) -> Result<Rc<RefCell<dyn WindowUserState>>, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(self.win[idx].user_state.clone())
    }

    pub fn frame(&self, handle: WindowHandle) -> Rc<dyn WindowFrame> {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        self.win[idx].frame.clone()
    }

    pub fn try_frame(&self, handle: WindowHandle) -> Result<Rc<dyn WindowFrame>, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(self.win[idx].frame.clone())
    }

    pub fn frame_style(&self, handle: WindowHandle) -> Rc<dyn WindowFrameStyle> {
        let idx = self.try_handle_idx(handle).expect("valid idx");
        self.win[idx].frame_style.clone()
    }

    pub fn try_frame_style(&self, handle: WindowHandle) -> Result<Rc<dyn WindowFrameStyle>, Error> {
        let idx = self.try_handle_idx(handle)?;
        Ok(self.win[idx].frame_style.clone())
    }
}

impl<T> WindowsState<T>
where
    T: Window,
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
        f: impl FnOnce(&mut WindowsState<T>, Position, WindowHandle, usize) -> R,
    ) -> Option<(WindowHandle, R)> {
        let pos = self.screen_to_window_pos(pos);

        // focus and front window
        let mut it = self.win.iter().enumerate().rev();
        loop {
            let Some((idx_win, win)) = it.next() else {
                break;
            };
            if win.state.borrow().area.contains(pos) {
                let handle = self.idx_handle(idx_win);
                let r = f(self, pos, handle, idx_win);
                return Some((handle, r));
            }
        }
        None
    }

    // // transform from 0-based coordinates relative to windows area
    // // into the true windows coordinates which are relative to windows.zero
    // //
    // // this is necessary to enable negative coordinates for windows.
    // fn shift_0_in(&self, rect: Rect) -> Rect {
    //     Rect::new(
    //         rect.x + self.zero_offset.x,
    //         rect.y + self.zero_offset.y,
    //         rect.width,
    //         rect.height,
    //     )
    // }
    //
    // // transform from 0-based coordinates relative to windows area
    // // into the true windows coordinates which are relative to windows.zero
    // //
    // // this is necessary to enable negative coordinates for windows.
    // #[allow(dead_code)]
    // fn shift_0_in_pos(&self, pos: Position) -> Position {
    //     let x = pos.x + self.zero_offset.x;
    //     let y = pos.y + self.zero_offset.y;
    //     Position::new(x, y)
    // }

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

impl<T: Debug> Debug for WinStruct<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinStruct")
            .field("win", &self.win)
            .field("state", &self.state)
            .field("user_state", &"..dyn..")
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
