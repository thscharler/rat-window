use crate::util::{copy_buffer, revert_style};
use crate::win_flags::WinFlags;
use crate::windows::WinHandle;
use rat_event::{ct_event, HandleEvent, Outcome, Regular};
use rat_focus::HasFocus;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::prelude::{BlockExt, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget, WidgetRef};
use std::collections::HashMap;
use std::mem;

#[derive(Debug, Default)]
pub struct DecoOne {
    block: Option<Block<'static>>,
    title_style: Style,
    title_alignment: Alignment,
    focus_style: Option<Style>,
}

#[derive(Debug, Default)]
pub struct DecoOneState {
    /// Temporary buffer for rendering.
    tmp: Buffer,

    /// Render offset.
    offset: Position,

    /// Window metadata.
    meta: HashMap<WinHandle, DecoMeta>,
    /// Rendering order. Back to front.
    order: Vec<WinHandle>,

    /// Currently moving window
    move_handle: Option<WinHandle>,
    /// Offset mouse cursor to window origin.
    move_offset: Position,
}

#[derive(Debug)]
struct DecoMeta {
    area: Rect,
    widget_area: Rect,

    close_area: Rect,
    move_area: Rect,
    resize_left_area: Rect,
    resize_right_area: Rect,
    resize_bottom_left_area: Rect,
    resize_bottom_area: Rect,
    resize_bottom_right_area: Rect,

    flags: WinFlags,
}

impl Default for DecoMeta {
    fn default() -> Self {
        Self {
            area: Default::default(),
            widget_area: Default::default(),
            close_area: Default::default(),
            move_area: Default::default(),
            resize_left_area: Default::default(),
            resize_right_area: Default::default(),
            resize_bottom_left_area: Default::default(),
            resize_bottom_area: Default::default(),
            resize_bottom_right_area: Default::default(),
            flags: Default::default(),
        }
    }
}

impl DecoOne {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    pub fn title_alignment(mut self, align: Alignment) -> Self {
        self.title_alignment = align;
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }
}

impl DecoOne {
    pub fn prepare_window(&self, handle: WinHandle, flags: WinFlags, state: &mut DecoOneState) {
        let win_area = state.window_area(handle);

        let meta = state.meta.get_mut(&handle).expect("window");
        meta.flags = flags;

        meta.widget_area = self.block.inner_if_some(win_area);
        if meta.widget_area.y == win_area.y {
            // need title
            meta.widget_area.y += 1;
        }
        meta.close_area = if meta.flags.closeable {
            Rect::new(win_area.right().saturating_sub(4), win_area.top(), 3, 1)
        } else {
            Rect::default()
        };
        meta.move_area = if meta.flags.moveable {
            Rect::new(win_area.left(), win_area.top(), win_area.width, 1)
        } else {
            Rect::default()
        };
        meta.resize_left_area = if meta.flags.resizable {
            Rect::new(
                win_area.left(),
                win_area.top() + 1,
                1,
                win_area.height.saturating_sub(2),
            )
        } else {
            Rect::default()
        };
        meta.resize_right_area = if meta.flags.resizable {
            Rect::new(
                win_area.right().saturating_sub(1),
                win_area.top() + 1,
                1,
                win_area.height.saturating_sub(2),
            )
        } else {
            Rect::default()
        };
        meta.resize_bottom_left_area = if meta.flags.resizable {
            Rect::new(win_area.left(), win_area.bottom().saturating_sub(1), 1, 1)
        } else {
            Rect::default()
        };
        meta.resize_bottom_area = if meta.flags.resizable {
            Rect::new(
                win_area.left() + 1,
                win_area.bottom().saturating_sub(1),
                win_area.width.saturating_sub(2),
                1,
            )
        } else {
            Rect::default()
        };
        meta.resize_bottom_right_area = if meta.flags.resizable {
            Rect::new(
                win_area.right().saturating_sub(1),
                win_area.bottom().saturating_sub(1),
                1,
                1,
            )
        } else {
            Rect::default()
        };
    }

    pub fn get_buffer(&self, handle: WinHandle, state: &mut DecoOneState) -> Buffer {
        let mut tmp = mem::take(&mut state.tmp);
        tmp.resize(state.window_area(handle));
        tmp
    }

    pub fn set_buffer(&self, tmp: Buffer, state: &mut DecoOneState) {
        state.tmp = tmp;
    }

    pub fn render_window(&mut self, handle: WinHandle, tmp: &mut Buffer, state: &mut DecoOneState) {
        let meta = state.meta.get(&handle).expect("window");

        let focus = meta.flags.focus.get();
        let style = if focus {
            self.focus_style.unwrap_or(revert_style(self.title_style))
        } else {
            self.title_style
        };

        // title
        let area = meta.area;

        // render border
        self.block.as_ref().render_ref(area, tmp);

        // complete title bar
        for x in area.left() + 1..area.right().saturating_sub(1) {
            if let Some(cell) = &mut tmp.cell_mut(Position::new(x, area.top())) {
                cell.set_style(style);
                cell.set_symbol(" ");
            }
        }

        // title text
        let title_area = Rect::new(
            area.left() + 1,
            area.top(),
            if meta.flags.closeable {
                meta.close_area.x - (area.x + 1)
            } else {
                area.width - 2
            },
            1,
        );
        Text::from(meta.flags.title.as_str())
            .alignment(self.title_alignment)
            .render(title_area, tmp);

        if meta.flags.closeable {
            Span::from(" \u{2A2F} ").render(meta.close_area, tmp);
        }
    }

    pub fn shift_clip_copy(
        &self,
        tmp: &mut Buffer,
        screen_area: Rect,
        screen_buf: &mut Buffer,
        state: &mut DecoOneState,
    ) {
        copy_buffer(tmp, state.offset, screen_area, screen_buf);
    }
}

impl DecoOneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn offset(&self) -> Position {
        self.offset
    }

    pub fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    pub fn insert(&mut self, handle: WinHandle) {
        self.meta.insert(handle, DecoMeta::default());
        self.order.push(handle);
    }

    pub fn remove(&mut self, handle: WinHandle) {
        self.meta.remove(&handle);
        self.order.retain(|v| *v != handle);
    }

    pub fn window_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").area
    }

    pub fn set_window_area(&mut self, handle: WinHandle, area: Rect) {
        self.meta.get_mut(&handle).expect("window").area = area;
    }

    pub fn window_widget_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").widget_area
    }

    pub fn set_window_widget_area(&mut self, handle: WinHandle, area: Rect) {
        self.meta.get_mut(&handle).expect("window").widget_area = area;
    }

    pub fn window_is_focused(&self, handle: WinHandle) -> bool {
        self.meta
            .get(&handle)
            .expect("window")
            .flags
            .focus
            .is_focused()
    }

    pub fn focus_window(&mut self, handle: WinHandle) -> bool {
        for meta in self.meta.values_mut() {
            meta.flags.focus.clear();
        }
        self.meta
            .get(&handle)
            .expect("window")
            .flags
            .focus
            .set(true);
        true
    }

    pub fn focused_window(&self) -> Option<WinHandle> {
        for handle in self.order.iter().rev().copied() {
            if self.window_is_focused(handle) {
                return Some(handle);
            }
        }
        None
    }

    pub fn windows(&self) -> Vec<WinHandle> {
        self.order.clone()
    }

    pub fn window_at(&self, position: Position) -> Option<WinHandle> {
        for handle in self.order.iter().rev().copied() {
            let area = self.window_area(handle);
            if area.contains(position) {
                return Some(handle);
            }
        }
        None
    }

    pub fn window_to_front(&mut self, handle: WinHandle) -> bool {
        self.order.retain(|v| *v != handle);
        self.order.push(handle);
        true
    }
}

// TODO: DecoOneOutcome

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        match event {
            ct_event!(mouse down Left for x,y) => {
                if let Some(handle) = self.window_at(Position::new(*x, *y)) {
                    if let Some(meta) = self.meta.get(&handle) {
                        if meta.move_area.contains(Position::new(*x, *y)) {
                            self.move_handle = Some(handle);
                            self.move_offset =
                                Position::new(*x - meta.move_area.x, *y - meta.move_area.y);
                            Outcome::Changed
                        } else {
                            Outcome::Continue
                        }
                    } else {
                        Outcome::Continue
                    }
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse drag Left for x,y) => {
                if let Some(handle) = self.move_handle {
                    let mut area = self.window_area(handle);

                    let move_offset = self.move_offset;
                    area.x = x.saturating_sub(move_offset.x);
                    area.y = y.saturating_sub(move_offset.y);

                    self.set_window_area(handle, area);
                    Outcome::Changed
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse up Left for _x,_y) | ct_event!(mouse moved for _x,_y) => {
                self.move_handle = None;
                self.move_offset = Position::default();
                Outcome::Continue
            }

            _ => Outcome::Continue,
        }
    }
}
