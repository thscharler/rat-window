use crate::deco_one::OneDecoration;
use crate::utils::{copy_buffer, fill_buf_area};
use crate::{Error, Window, WindowState};
use bimap::BiMap;
use log::debug;
use rat_event::{ct_event, HandleEvent, MouseOnly, Outcome, Regular};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::{StatefulWidget, Style};
use ratatui::widgets::Block;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// Handle returned for an added window. Used as a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(usize);

/// Window handler
#[derive(Debug)]
pub struct Windows<'a, T>
where
    T: Window,
{
    block: Option<Block<'a>>,
    title_style: Option<Style>,
    title_alignment: Option<Alignment>,
    focus_style: Option<Style>,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
pub struct WindowsState<T>
where
    T: Window,
{
    // last rendered area for windowing.
    // read-only
    pub area: Rect,

    // offset of the window pane.
    // there are no negative coordinates.
    // this offset makes good for it.
    pub zero: (u16, u16),

    // max handle
    max_id: usize,

    // window handles
    win_handle: BiMap<WindowHandle, usize>,
    // window widget
    win: Vec<T>,
    // areas. x,y have zero added.
    win_area: Vec<Rect>,
    // other window state
    win_state: Vec<RefCell<WindowState>>,

    // mouse stuff
    mouse: WinMouseFlags,
}

#[derive(Debug, Default, Clone, Copy)]
struct WinMouseFlags {
    drag_ref: (u16, u16),
    drag_zero: (u16, u16),
    drag_win: Option<usize>,
    drag_area: Option<usize>,
}

impl<'a, T> Default for Windows<'a, T>
where
    T: Window,
{
    fn default() -> Self {
        Self {
            block: None,
            title_style: None,
            title_alignment: None,
            focus_style: None,
            _phantom: Default::default(),
        }
    }
}

impl<'a, T> Windows<'a, T>
where
    T: Window,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn focus_style(mut self, focused: Style) -> Self {
        self.focus_style = Some(focused);
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = Some(style);
        self
    }

    pub fn title_alignment(mut self, align: Alignment) -> Self {
        self.title_alignment = Some(align);
        self
    }
}

impl<'a, T> StatefulWidget for Windows<'a, T>
where
    T: Window,
{
    type State = WindowsState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;

        // necessary buffer
        let mut tmp_area = Rect::new(state.zero.0, state.zero.1, 0, 0);
        for area in state.win_area.iter() {
            tmp_area = tmp_area.union(*area);
        }

        let mut tmp = Buffer::empty(tmp_area);

        for ((win, state), area) in state
            .win
            .iter_mut()
            .zip(state.win_state.iter())
            .zip(state.win_area.iter())
        {
            let mut state = state.borrow_mut();

            state.closeable = win.is_closeable();
            state.moveable = win.is_moveable();
            state.resizable = win.is_resizable();
            state.modal = win.is_modal();

            // Clear out window area
            fill_buf_area(&mut tmp, *area, " ", state.style);

            // decorations
            OneDecoration::new()
                .block(self.block.clone())
                .title(win.title())
                .title_style(self.title_style)
                .title_alignment(self.title_alignment)
                .focus_style(self.focus_style)
                .render(*area, &mut tmp, state.deref_mut());

            // content
            win.render(state.inner, &mut tmp);
        }

        copy_buffer(tmp, state.zero.0, state.zero.1, area, buf);
    }
}

impl<T> Default for WindowsState<T>
where
    T: Window,
{
    fn default() -> Self {
        Self {
            area: Default::default(),
            zero: (0, 0),
            max_id: 0,
            win_handle: Default::default(),
            win: vec![],
            win_area: vec![],
            win_state: vec![],
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
                if let Some((h, _)) = self.to_front_at((*x, *y)) {
                    r = self.focus_window(h).into();
                }

                // Test for some draggable area.
                self.at_hit((*x, *y), |w, _handle, idx_win| {
                    debug!("HITHIT {:?} {:?}", _handle, idx_win);
                    let win = w.win_state[idx_win].borrow();
                    debug!("  => state {:?}", win);
                    for (idx_area, area) in win.areas.iter().enumerate() {
                        if area.contains((*x, *y).into()) {
                            debug!("  => area {}", idx_area);
                            w.mouse.drag_zero = (*x, *y);
                            w.mouse.drag_ref = (area.x, area.y);
                            w.mouse.drag_win = Some(idx_win);
                            w.mouse.drag_area = Some(idx_area);
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
                    Some(WindowState::MOVE) => {
                        let area = &mut self.win_area[win_idx];
                        area.x =
                            (self.mouse.drag_ref.0 + *x).saturating_sub(self.mouse.drag_zero.0);
                        area.y =
                            (self.mouse.drag_ref.1 + *y).saturating_sub(self.mouse.drag_zero.1);
                        Outcome::Changed
                    }
                    _ => Outcome::Continue,
                }
            }
            ct_event!(mouse up Left for _x, _y) | ct_event!(mouse moved for _x, _y) => {
                self.mouse.drag_zero = self.zero;
                self.mouse.drag_ref = self.zero;
                self.mouse.drag_win = None;
                self.mouse.drag_area = None;
                Outcome::Changed
            }
            _ => Outcome::Continue,
        }
    }
}

impl<T> WindowsState<T>
where
    T: Window,
{
    /// Show with bounds.
    ///
    /// Bounds are relative to the zero-point.
    pub fn show_at(&mut self, w: T, bounds: Rect) -> WindowHandle {
        let handle = self.new_handle();
        let idx = self.win.len();

        self.win_handle
            .insert_no_overwrite(handle, idx)
            .expect("no duplicate");
        self.win_state.push(RefCell::new(WindowState {
            area: Default::default(),
            inner: Default::default(),
            style: Default::default(),
            areas: [Rect::default(); 11],
            focus: w.focus(),
            modal: w.is_modal(),
            closeable: w.is_closeable(),
            resizable: w.is_resizable(),
            moveable: w.is_moveable(),
        }));
        self.win_area.push(self.shift_0_in(bounds));
        self.win.push(w);
        handle
    }

    /// Show.
    ///
    /// Fills all the visible area.
    pub fn show(&mut self, w: T) -> WindowHandle {
        self.show_at(w, self.area)
    }

    /// Move window at position to the front.
    ///
    /// This takes screen coordinates.
    ///
    /// __Panic__
    ///
    /// Panics if pos is not in bounds of the windows area.
    pub fn focus_window_at(&mut self, pos: (u16, u16)) -> Option<(WindowHandle, bool)> {
        self.at_hit(pos, |w, handle, _| {
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

        let old_focus = self.win[idx_win].focus().get();

        for (idx, win) in self.win.iter().enumerate() {
            if idx_win == idx {
                win.focus().set(true);
            } else {
                win.focus().set(false);
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
    pub fn to_front_at(&mut self, pos: (u16, u16)) -> Option<(WindowHandle, bool)> {
        self.at_hit(pos, |w, handle, _| {
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
        let area = self.win_area.remove(idx_win);
        let state = self.win_state.remove(idx_win);

        // correct handle mappings, shift left
        for cor in idx_win + 1..=max_idx {
            let (h, _) = self.win_handle.remove_by_right(&cor).expect("valid win");
            self.win_handle
                .insert_no_overwrite(h, cor - 1)
                .expect("no duplicates")
        }

        // reinstate
        self.win.push(win);
        self.win_area.push(area);
        self.win_state.push(state);

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
        pos: (u16, u16),
        f: impl FnOnce(&mut WindowsState<T>, WindowHandle, usize) -> R,
    ) -> Option<(WindowHandle, R)> {
        let pos = self.shift_in_pos(pos);

        // focus and front window
        let mut it = self.win_area.iter().enumerate().rev();
        loop {
            let Some((idx_win, win_area)) = it.next() else {
                break;
            };
            if win_area.contains(pos.into()) {
                let handle = self.idx_handle(idx_win);
                let r = f(self, handle, idx_win);
                return Some((handle, r));
            }
        }
        None
    }

    // transform from 0-based coordinates relative to windows area
    // into the true windows coordinates which are relative to windows.zero
    //
    // this is necessary to enable negative coordinates for windows.
    fn shift_0_in(&self, mut rect: Rect) -> Rect {
        rect.x += self.zero.0;
        rect.y += self.zero.1;
        rect
    }

    // transform from 0-based coordinates relative to windows area
    // into the true windows coordinates which are relative to windows.zero
    //
    // this is necessary to enable negative coordinates for windows.
    #[allow(dead_code)]
    fn shift_0_in_pos(&self, mut pos: (u16, u16)) -> (u16, u16) {
        pos.0 += self.zero.0;
        pos.1 += self.zero.1;
        pos
    }

    // transformation from terminal-space to windows-space
    #[allow(dead_code)]
    fn shift_in(&self, mut rect: Rect) -> Rect {
        rect.x -= self.area.x;
        rect.x -= self.area.y;
        rect.x += self.zero.0;
        rect.y += self.zero.1;
        rect
    }

    // transformation from terminal-space to windows-space
    fn shift_in_pos(&self, mut pos: (u16, u16)) -> (u16, u16) {
        pos.0 -= self.area.x;
        pos.1 -= self.area.y;
        pos.0 += self.zero.0;
        pos.1 += self.zero.1;
        pos
    }
}

impl WinMouseFlags {
    pub(crate) fn clear(&mut self) {
        self.drag_zero = (0, 0);
        self.drag_ref = (0, 0);
        self.drag_area = None;
        self.drag_win = None;
    }
}
