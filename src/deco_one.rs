use crate::util::{copy_buffer, revert_style};
use crate::win_flags::WinFlags;
use crate::windows::WinHandle;
use rat_event::{ct_event, HandleEvent, Outcome, Regular};
use rat_focus::HasFocus;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Position, Rect, Size};
use ratatui::prelude::{BlockExt, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget, WidgetRef};
use std::cmp::max;
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
    /// View area in screen coordinates.
    area: Rect,

    /// Window metadata.
    meta: HashMap<WinHandle, DecoMeta>,
    /// Rendering order. Back to front.
    order: Vec<WinHandle>,

    /// Currently dragged window
    drag_action: DragAction,
    drag_handle: Option<WinHandle>,
    /// Offset mouse cursor to window origin.
    drag_offset: (u16, u16),

    /// resize areas. when inside a resize to b while moving.
    resize_areas: Vec<(Rect, Rect)>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum DragAction {
    #[default]
    None,
    Move,
    ResizeLeft,
    ResizeRight,
    ResizeBottomLeft,
    ResizeBottom,
    ResizeBottomRight,
}

#[derive(Debug)]
struct DecoMeta {
    base_size: Size,

    window_area: Rect,
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
            base_size: Default::default(),
            window_area: Default::default(),
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
        let area = meta.window_area;

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
                area.width.saturating_sub(2)
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

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.calculate_hot();
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
        self.meta.get(&handle).expect("window").window_area
    }

    pub fn set_window_area(&mut self, handle: WinHandle, area: Rect) {
        self.meta.get_mut(&handle).expect("window").window_area = area;
    }

    pub fn base_size(&self, handle: WinHandle) -> Size {
        self.meta.get(&handle).expect("window").base_size
    }

    pub fn set_base_size(&mut self, handle: WinHandle, size: Size) {
        self.meta.get_mut(&handle).expect("window").base_size = size;
    }

    pub fn window_widget_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").widget_area
    }

    pub fn set_window_widget_area(&mut self, handle: WinHandle, area: Rect) {
        self.meta.get_mut(&handle).expect("window").widget_area = area;
    }

    pub fn is_window_focused(&self, handle: WinHandle) -> bool {
        self.meta
            .get(&handle)
            .expect("window")
            .flags
            .focus
            .is_focused()
    }

    pub fn set_focused_window(&mut self, handle: WinHandle) -> bool {
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
            if self.is_window_focused(handle) {
                return Some(handle);
            }
        }
        None
    }

    /// Returns a list of handles in render order bottom-z to top-z.
    pub fn windows(&self) -> Vec<WinHandle> {
        self.order.clone()
    }

    /// Move a window to front.
    pub fn window_to_front(&mut self, handle: WinHandle) -> bool {
        if self.order.last() == Some(&handle) {
            false
        } else {
            self.order.retain(|v| *v != handle);
            self.order.push(handle);
            true
        }
    }
}

impl DecoOneState {
    /// Window at the given __window__ position.
    pub fn window_at(&self, pos: Position) -> Option<WinHandle> {
        for handle in self.order.iter().rev().copied() {
            let area = self.window_area(handle);
            if area.contains(pos) {
                return Some(handle);
            }
        }
        None
    }

    /// Translate screen coordinates to window coordinates.
    pub fn screen_to_win(&self, pos: Position) -> Option<Position> {
        if pos.x + self.offset.x >= self.area.x && pos.y + self.offset.y >= self.area.y {
            Some(Position::new(
                (pos.x + self.offset.x).saturating_sub(self.area.x),
                (pos.y + self.offset.y).saturating_sub(self.area.y),
            ))
        } else {
            None
        }
    }

    /// Translate window coordinates to screen coordinates
    pub fn win_to_screen(&self, pos: Position) -> Option<Position> {
        if pos.x + self.area.x >= self.offset.x && pos.y + self.area.y >= self.offset.y {
            Some(Position::new(
                (pos.x + self.area.x).saturating_sub(self.offset.x),
                (pos.y + self.area.y).saturating_sub(self.offset.y),
            ))
        } else {
            None
        }
    }
}

impl DecoOneState {
    fn calculate_hot(&mut self) {
        self.resize_areas.clear();

        let area = Rect::from((
            self.screen_to_win(self.area.as_position())
                .expect("valid_pos"),
            self.area.as_size(),
        ));

        let w_clip = area.width * 2 / 5;
        let h_clip = area.width * 2 / 5;

        // snap-click to top
        self.resize_areas.push((
            Rect::new(area.x + w_clip, area.y, area.width - 2 * w_clip, 2),
            Rect::new(area.x, area.y, area.width, area.height / 2),
        ));
        // snap-click to bottom
        self.resize_areas.push((
            Rect::new(
                area.x + w_clip,
                (area.y + area.height).saturating_sub(1),
                area.width - 2 * w_clip,
                2,
            ),
            Rect::new(
                area.x,
                area.y + area.height / 2,
                area.width,
                area.height - area.height / 2,
            ),
        ))
    }

    fn calculate_resize_left(&self, mut area: Rect, pos: Position) -> Rect {
        let right = area.x + area.width;
        area.x = pos.x;
        if area.x < self.offset.x {
            area.x = self.offset.x;
        } else if area.x >= right.saturating_sub(2) {
            area.x = right.saturating_sub(2);
        }
        area.width = right.saturating_sub(area.x);
        area
    }

    fn calculate_resize_right(&self, mut area: Rect, pos: Position, max_x: u16) -> Rect {
        area.width = pos.x.saturating_sub(area.x);
        if area.width < 2 {
            area.width = 2;
        }
        if area.x + area.width >= max_x {
            area.width = max_x.saturating_sub(area.x) + 1;
        }
        area
    }

    fn calculate_resize_bottom(&self, mut area: Rect, pos: Position, max_y: u16) -> Rect {
        area.height = pos.y.saturating_sub(area.y);
        if area.height < 2 {
            area.height = 2;
        }
        if area.y + area.height >= max_y {
            area.height = max_y.saturating_sub(area.y) + 1;
        }
        area
    }

    fn calculate_move(
        &self,
        mut win_area: Rect,
        base_size: Size,
        pos: Position,
        max: (u16, u16),
    ) -> Rect {
        win_area.x = pos.x.saturating_sub(self.drag_offset.0);
        win_area.y = pos.y.saturating_sub(self.drag_offset.1);
        win_area.width = base_size.width;
        win_area.height = base_size.height;

        let mut hit_hot = false;
        for (hot_area, resize_to) in self.resize_areas.iter() {
            if hot_area.contains(win_area.as_position()) {
                hit_hot = true;
                win_area = *resize_to;
            }
        }
        if !hit_hot {
            if win_area.y < self.offset.y {
                win_area.y = self.offset.y;
            } else if win_area.y >= max.1 {
                win_area.y = max.1;
            }
            if win_area.x + win_area.width < self.offset.x {
                win_area.x = self.offset.x.saturating_sub(win_area.width);
            }
            if win_area.x >= max.0 {
                win_area.x = max.0;
            }
        }

        win_area
    }
}

// TODO: DecoOneOutcome

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        match event {
            ct_event!(mouse down Left for x,y) => {
                let pos = Position::new(*x, *y);
                if let Some(handle) = self.window_at(pos) {
                    let r0 = if let Some(handle) = self.window_at(pos) {
                        self.window_to_front(handle).into()
                    } else {
                        Outcome::Continue
                    };

                    let r1 = if let Some(meta) = self.meta.get(&handle) {
                        if meta.move_area.contains(pos) {
                            self.drag_action = DragAction::Move;
                            self.drag_handle = Some(handle);
                            if meta.window_area.as_size() != meta.base_size {
                                self.drag_offset = (0, 0).into();
                            } else {
                                self.drag_offset =
                                    (*x - meta.move_area.x, *y - meta.move_area.y).into();
                            }
                            Outcome::Changed
                        } else if meta.resize_right_area.contains(pos) {
                            self.drag_action = DragAction::ResizeRight;
                            self.drag_handle = Some(handle);
                            Outcome::Changed
                        } else if meta.resize_bottom_right_area.contains(pos) {
                            self.drag_action = DragAction::ResizeBottomRight;
                            self.drag_handle = Some(handle);
                            Outcome::Changed
                        } else if meta.resize_bottom_area.contains(pos) {
                            self.drag_action = DragAction::ResizeBottom;
                            self.drag_handle = Some(handle);
                            Outcome::Changed
                        } else if meta.resize_bottom_left_area.contains(pos) {
                            self.drag_action = DragAction::ResizeBottomLeft;
                            self.drag_handle = Some(handle);
                            Outcome::Changed
                        } else if meta.resize_left_area.contains(pos) {
                            self.drag_action = DragAction::ResizeLeft;
                            self.drag_handle = Some(handle);
                            Outcome::Changed
                        } else {
                            Outcome::Continue
                        }
                    } else {
                        Outcome::Continue
                    };

                    max(r0, r1)
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse up Left for x,y) => {
                if self.drag_handle.is_some() {
                    self.drag_handle = None;
                    self.drag_action = DragAction::default();
                    self.drag_offset = Default::default();
                    Outcome::Changed
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse drag Left for x,y) => {
                if let Some(handle) = self.drag_handle {
                    let max_x = (self.offset.x + self.area.width).saturating_sub(1);
                    let max_y = (self.offset.y + self.area.height).saturating_sub(1);
                    let base_size = self.base_size(handle);

                    let mut new = self.window_area(handle);
                    new = match self.drag_action {
                        DragAction::None => new,
                        DragAction::Move => self.calculate_move(
                            new,
                            base_size,
                            Position::new(*x, *y),
                            (max_x, max_y),
                        ),
                        DragAction::ResizeLeft => {
                            self.calculate_resize_left(new, Position::new(*x, *y))
                        }
                        DragAction::ResizeRight => {
                            self.calculate_resize_right(new, Position::new(*x, *y), max_x)
                        }
                        DragAction::ResizeBottomLeft => {
                            new = self.calculate_resize_left(new, Position::new(*x, *y));
                            self.calculate_resize_bottom(new, Position::new(*x, *y), max_y)
                        }
                        DragAction::ResizeBottom => {
                            self.calculate_resize_bottom(new, Position::new(*x, *y), max_y)
                        }
                        DragAction::ResizeBottomRight => {
                            new = self.calculate_resize_right(new, Position::new(*x, *y), max_x);
                            self.calculate_resize_bottom(new, Position::new(*x, *y), max_y)
                        }
                    };
                    self.set_window_area(handle, new);
                    Outcome::Changed
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse moved for _x,_y) => {
                // reset on fail otherwise
                self.drag_handle = None;
                self.drag_action = DragAction::default();
                self.drag_offset = Default::default();
                Outcome::Continue
            }

            _ => Outcome::Continue,
        }
    }
}
