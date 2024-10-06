use crate::deco::One;
use crate::deco_one::OneStyle;
use crate::utils::{copy_buffer, fill_buf_area};
use crate::window_style::{WindowFrame, WindowFrameStyle};
use crate::{Error, Window, WindowState};
use anyhow::anyhow;
use bimap::BiMap;
use log::debug;
use rat_event::{ct_event, HandleEvent, MouseOnly, Outcome, Regular};
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
    win: Vec<T>,
    // areas. x,y have zero added.
    win_area: Vec<Rect>,
    // other window state
    win_state: Vec<Rc<RefCell<WindowState>>>,
    // window style
    win_style: Vec<(Rc<dyn WindowFrame>, Rc<dyn WindowFrameStyle>)>,

    // mouse stuff
    mouse: WinMouseFlags,
}

#[derive(Debug, Default, Clone, Copy)]
struct WinMouseFlags {
    drag_base: Option<Rect>,
    drag_zero: Option<Position>,
    drag_win: Option<usize>,
    drag_area: Option<usize>,
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
        let mut it = state.win_area.iter();
        loop {
            let Some(area) = it.next() else {
                break;
            };

            if let Some(tmp_area) = tmp_area.as_mut() {
                *tmp_area = tmp_area.union(*area);
            } else {
                tmp_area = Some(*area);
            }
            debug!("build tmp_area {:?} -> {:?}", area, tmp_area);
        }
        let tmp_area = tmp_area.unwrap_or_default();
        debug!("final tmp_area {:?}", tmp_area);

        // buffer is constructed with windows coordinates
        let mut tmp = Buffer::empty(Rect::new(
            tmp_area.x,
            tmp_area.y,
            tmp_area.width,
            tmp_area.height,
        ));

        debug!("tmp area {:?}", tmp_area);

        for (((win, win_state), win_area), win_style) in //
            state
                .win
                .iter_mut()
                .zip(state.win_state.iter())
                .zip(state.win_area.iter())
                .zip(state.win_style.iter())
        {
            win_state.borrow_mut().title = win.title().unwrap_or_default().into();
            win_state.borrow_mut().closeable = win.is_closeable();
            win_state.borrow_mut().moveable = win.is_moveable();
            win_state.borrow_mut().resizable = win.is_resizable();
            win_state.borrow_mut().modal = win.is_modal();

            // win rect in windows coordinates
            let win_area_rect = Rect::new(win_area.x, win_area.y, win_area.width, win_area.height);

            // Clear out window area
            fill_buf_area(&mut tmp, win_area_rect, " ", Style::default());

            // decorations
            let frame = win_style.0.as_ref();
            let frame_style_clone = win_style.1.clone();
            let win_state_clone = win_state.clone();
            frame.render_ref(
                win_area_rect,
                &mut tmp,
                &mut (win_state_clone, frame_style_clone),
            );

            // content
            win.render(win_state.borrow().inner, &mut tmp);
        }

        copy_buffer(tmp, state.zero_offset, area, buf);
    }
}

impl<T> Debug for WindowsState<T>
where
    T: Window,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsState")
            .field("area", &self.area)
            .field("zero", &self.zero_offset)
            .field("max_id", &self.max_id)
            .field("win_handle", &self.win_handle)
            .field("win", &self.win)
            .field("win_area", &self.win_area)
            .field("win_state", &self.win_state)
            .field("win_style", &"..dyn something")
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
            win_area: vec![],
            win_state: vec![],
            win_style: vec![],
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
                debug!("** RAW CLICK {} {}", x, y);
                self.at_hit(Position::new(*x, *y), |w, pos, _handle, idx_win| {
                    let win = w.win_state[idx_win].borrow();
                    for (idx_area, area) in win.areas.iter().enumerate() {
                        if area.contains(pos.into()) {
                            w.mouse.drag_zero = Some(pos);
                            w.mouse.drag_base = Some(win.area);
                            w.mouse.drag_win = Some(idx_win);
                            w.mouse.drag_area = Some(idx_area);
                            debug!(
                                "start drag ZERO {:?} WIN-AREA {:?} WIN {:?}",
                                w.mouse.drag_zero, area, w.mouse.drag_base
                            );
                            break;
                        }
                    }
                });

                r
            }
            ct_event!(mouse drag Left for x,y) => 'f: {
                debug!("** RAW DRAG {} {}", x, y);
                let Some(win_idx) = self.mouse.drag_win else {
                    break 'f Outcome::Continue;
                };

                match self.mouse.drag_area {
                    None => Outcome::Continue,
                    Some(WindowState::MOVE) => {
                        let pos = self.shift_in_pos(Position::new(*x, *y));
                        let area = &mut self.win_area[win_idx];
                        let zero = self.mouse.drag_zero.expect("zero");
                        let base = self.mouse.drag_base.expect("base");
                        debug!(
                            "dragging area {:?}:: base {:?} + pos {:?} - zero {:?}",
                            area,
                            (base.x, base.y),
                            (pos.x, pos.y),
                            (zero.x, zero.y)
                        );
                        area.x = (base.x + pos.x).saturating_sub(zero.x);
                        area.y = (base.y + pos.y).saturating_sub(zero.y);
                        debug!("after dragging {:?}", area);
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
    pub fn show_at(&mut self, w: T, bounds: Rect) -> WindowHandle {
        let handle = self.new_handle();
        let idx = self.win.len();

        self.win_handle
            .insert_no_overwrite(handle, idx)
            .expect("no duplicate");
        self.win_state.push(Rc::new(RefCell::new(WindowState {
            area: Default::default(),
            inner: Default::default(),
            areas: [Rect::default(); 11],
            focus: w.focus(),
            modal: w.is_modal(),
            closeable: w.is_closeable(),
            resizable: w.is_resizable(),
            moveable: w.is_moveable(),
            title: "".to_string(),
        })));
        self.win_style.push((
            Rc::clone(&self.default_deco),
            Rc::clone(&self.default_deco_style),
        ));
        debug!("SHOW WIN {:?}", self.shift_0_in(bounds));
        self.win_area.push(self.shift_0_in(bounds));
        self.win.push(w);
        handle
    }

    /// Show.
    ///
    /// Fills all the visible area.
    pub fn show(&mut self, w: T) -> WindowHandle {
        self.show_at(w, Rect::new(0, 0, self.area.width, self.area.height))
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
        let area = self.win_area.remove(idx_win);
        let state = self.win_state.remove(idx_win);
        let style = self.win_style.remove(idx_win);

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
        self.win_style.push(style);

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
        pos: Position,
        f: impl FnOnce(&mut WindowsState<T>, Position, WindowHandle, usize) -> R,
    ) -> Option<(WindowHandle, R)> {
        let pos = self.shift_in_pos(pos);

        // focus and front window
        let mut it = self.win_area.iter().enumerate().rev();
        loop {
            let Some((idx_win, win_area)) = it.next() else {
                break;
            };
            if win_area.contains(pos) {
                let handle = self.idx_handle(idx_win);
                let r = f(self, pos, handle, idx_win);
                return Some((handle, r));
            }
        }
        None
    }

    // transform from 0-based coordinates relative to windows area
    // into the true windows coordinates which are relative to windows.zero
    //
    // this is necessary to enable negative coordinates for windows.
    fn shift_0_in(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.x + self.zero_offset.x,
            rect.y + self.zero_offset.y,
            rect.width,
            rect.height,
        )
    }

    // transform from 0-based coordinates relative to windows area
    // into the true windows coordinates which are relative to windows.zero
    //
    // this is necessary to enable negative coordinates for windows.
    #[allow(dead_code)]
    fn shift_0_in_pos(&self, pos: Position) -> Position {
        let x = pos.x + self.zero_offset.x;
        let y = pos.y + self.zero_offset.y;
        Position::new(x, y)
    }

    // transformation from terminal-space to windows-space
    #[allow(dead_code)]
    fn shift_in(&self, rect: Rect) -> Rect {
        let x = (rect.x - self.area.x) + self.zero_offset.x;
        let y = (rect.y - self.area.y) + self.zero_offset.y;
        Rect::new(x, y, rect.width, rect.height)
    }

    // transformation from terminal-space to windows-space
    fn shift_in_pos(&self, pos: Position) -> Position {
        // debug!("{:#?}", anyhow!("fff").backtrace());
        // debug!(
        //     "== shift in {:?} area {:?} zero {:?}",
        //     pos, self.area, self.zero_offset
        // );
        let x = (pos.x - self.area.x) + self.zero_offset.x;
        let y = (pos.y - self.area.y) + self.zero_offset.y;
        Position::new(x, y)
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
