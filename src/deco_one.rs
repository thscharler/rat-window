use crate::util::{copy_buffer, revert_style};
use crate::win_flags::WinFlags;
use crate::windows::WinHandle;
use log::debug;
use rat_event::util::MouseFlags;
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
    /// View area in screen coordinates.
    area: Rect,

    /// Temporary buffer for rendering.
    tmp: Buffer,
    /// Render offset. All coordinates are shifted by this
    /// value before rendering.
    offset: Position,

    /// Window metadata.
    meta: HashMap<WinHandle, DecoMeta>,
    /// Rendering order. Back to front.
    order: Vec<WinHandle>,
    /// Currently dragged mode and window
    drag: Option<Drag>,

    /// snap to tile areas. when inside a resize to b during move.
    snap_areas: Vec<(Vec<Rect>, Rect)>,

    /// mouse flags
    mouse: MouseFlags,
}

#[derive(Debug, PartialEq, Eq)]
enum DragAction {
    Move,
    ResizeLeft,
    ResizeRight,
    ResizeBottomLeft,
    ResizeBottom,
    ResizeBottomRight,
}

#[derive(Debug)]
struct Drag {
    // drag what?
    action: DragAction,
    // window
    handle: WinHandle,
    // snap before the drag
    base_snap: Option<usize>,
    // offset window origin to mouse cursor.
    win_offset: (u16, u16),
}

#[derive(Debug)]
struct DecoMeta {
    // base-line size of the window.
    base_size: Rect,
    // currently snapped to this snap region.
    snapped_to: Option<usize>,
    // effective window size.
    window_area: Rect,
    // area for the window content.
    widget_area: Rect,

    // close icon
    close_area: Rect,
    // drag to move
    move_area: Rect,
    // drag to resize
    resize_left_area: Rect,
    resize_right_area: Rect,
    resize_bottom_left_area: Rect,
    resize_bottom_area: Rect,
    resize_bottom_right_area: Rect,

    // display parameters
    flags: WinFlags,
}

impl Drag {
    fn new_move(handle: WinHandle, snap: Option<usize>, offset: (u16, u16)) -> Self {
        Self {
            action: DragAction::Move,
            handle,
            base_snap: snap,
            win_offset: offset,
        }
    }

    fn new_resize(handle: WinHandle, snap: Option<usize>, action: DragAction) -> Self {
        Self {
            action,
            handle,
            base_snap: snap,
            win_offset: (0, 0),
        }
    }
}

impl Default for DecoMeta {
    fn default() -> Self {
        Self {
            base_size: Default::default(),
            snapped_to: None,
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

    /// Current offset used for rendering.
    pub fn offset(&self) -> Position {
        self.offset
    }

    /// Current offset used for rendering.
    pub fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    /// Current windows area.
    /// In __screen__ coordinates.
    pub fn area(&self) -> Rect {
        self.area
    }

    // todo: xxx
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.calculate_snaps();
    }

    /// Add a new window
    pub fn insert(&mut self, handle: WinHandle) {
        assert!(!self.meta.contains_key(&handle));
        self.meta.insert(handle, DecoMeta::default());
        self.order.push(handle);
    }

    /// Remove a window.
    pub fn remove(&mut self, handle: WinHandle) {
        self.meta.remove(&handle);
        self.order.retain(|v| *v != handle);
    }

    /// Active window area.
    pub fn window_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").window_area
    }

    /// Active window area.
    pub fn set_window_area(&mut self, handle: WinHandle, area: Rect) {
        self.meta.get_mut(&handle).expect("window").window_area = area;
    }

    /// Base area of the window when not snapped to a region.
    pub fn base_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").base_size
    }

    /// Base area of the window when not snapped to a region.
    pub fn set_base_size(&mut self, handle: WinHandle, size: Rect) {
        self.meta.get_mut(&handle).expect("window").base_size = size;
    }

    /// Area for the window content.
    pub fn window_widget_area(&self, handle: WinHandle) -> Rect {
        self.meta.get(&handle).expect("window").widget_area
    }

    /// Area for the window content.
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
    /// Calculate the snap areas.
    fn calculate_snaps(&mut self) {
        self.snap_areas.clear();

        let area = Rect::from((
            self.screen_to_win(self.area.as_position())
                .expect("valid_pos"),
            self.area.as_size(),
        ));

        let w_clip = area.width / 5;
        let h_clip = area.height / 5;

        // 0: left
        self.snap_areas.push((
            vec![Rect::new(
                area.x,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(area.x, area.y, area.width / 2, area.height),
        ));

        // 1: alt left
        self.snap_areas.push((
            vec![Rect::new(
                area.x + 1,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(area.x, area.y, area.width * 6 / 10, area.height),
        ));

        // 2: right
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(1),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area.x + area.width / 2,
                area.y,
                area.width - area.width / 2,
                area.height,
            ),
        ));

        // 3: alt right
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(2),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area.x + area.width * 4 / 10,
                area.y,
                area.width - area.width * 4 / 10,
                area.height,
            ),
        ));

        // 4: snap-click to top
        self.snap_areas.push((
            vec![Rect::new(
                area.x + w_clip,
                area.y,
                area.width - 2 * w_clip,
                1,
            )],
            Rect::new(area.x, area.y, area.width, area.height / 2),
        ));

        // 5: snap-click to bottom
        self.snap_areas.push((
            vec![Rect::new(
                area.x + w_clip,
                (area.y + area.height).saturating_sub(1),
                area.width - 2 * w_clip,
                1,
            )],
            Rect::new(
                area.x,
                area.y + area.height / 2,
                area.width,
                area.height - area.height / 2,
            ),
        ));

        // 6: top left
        self.snap_areas.push((
            vec![
                Rect::new(area.x, area.y, w_clip, 1),
                Rect::new(area.x, area.y, 1, h_clip),
            ],
            Rect::new(area.x, area.y, area.width / 2, area.height / 2),
        ));

        // 7: top right
        self.snap_areas.push((
            vec![
                Rect::new(
                    (area.x + area.width - w_clip).saturating_sub(1),
                    area.y,
                    w_clip,
                    1,
                ),
                Rect::new(
                    (area.x + area.width).saturating_sub(1), //
                    area.y,
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area.x + area.width / 2,
                area.y,
                area.width - area.width / 2,
                area.height / 2,
            ),
        ));

        // 8: bottom left
        self.snap_areas.push((
            vec![
                Rect::new(
                    area.x, //
                    (area.y + area.height).saturating_sub(1),
                    w_clip,
                    1,
                ),
                Rect::new(
                    area.x,
                    (area.y + area.height - h_clip).saturating_sub(1),
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area.x,
                area.y + area.height / 2,
                area.width / 2,
                area.height - area.height / 2,
            ),
        ));

        // 9: bottom right
        self.snap_areas.push((
            vec![
                Rect::new(
                    (area.x + area.width - w_clip).saturating_sub(1),
                    (area.y + area.height).saturating_sub(1),
                    w_clip,
                    1,
                ),
                Rect::new(
                    (area.x + area.width).saturating_sub(1),
                    (area.y + area.height - h_clip).saturating_sub(1),
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area.x + area.width / 2,
                area.y + area.height / 2,
                area.width - area.width / 2,
                area.height - area.height / 2,
            ),
        ));

        // 10: full area
        self.snap_areas.push((
            Vec::default(),
            Rect::from((
                self.screen_to_win(self.area.as_position()).expect("area"),
                self.area.as_size(),
            )),
        ));
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
    ) -> (Option<usize>, Rect) {
        // match a snap area?
        for (idx, (snap_area, resize_to)) in self.snap_areas.iter().enumerate() {
            if snap_area.iter().find(|v| v.contains(pos)).is_some() {
                return (Some(idx), *resize_to);
            }
        }

        let Some(drag) = &self.drag else {
            panic!("drag not active")
        };

        // regular move
        win_area.x = pos.x.saturating_sub(drag.win_offset.0);
        win_area.y = pos.y.saturating_sub(drag.win_offset.1);
        win_area.width = base_size.width;
        win_area.height = base_size.height;

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
        (None, win_area)
    }
}

impl DecoOneState {
    /// Start dragging.
    fn initiate_drag(&mut self, handle: WinHandle, pos: Position) -> bool {
        if let Some(meta) = self.meta.get(&handle) {
            if meta.move_area.contains(pos) {
                self.drag = Some(Drag::new_move(
                    handle,
                    meta.snapped_to,
                    if meta.window_area.as_size() != meta.base_size.as_size() {
                        (0, 0).into()
                    } else {
                        (pos.x - meta.move_area.x, pos.y - meta.move_area.y).into()
                    },
                ));
                true
            } else if meta.resize_right_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    meta.snapped_to,
                    DragAction::ResizeRight,
                ));
                true
            } else if meta.resize_bottom_right_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    meta.snapped_to,
                    DragAction::ResizeBottomRight,
                ));
                true
            } else if meta.resize_bottom_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    meta.snapped_to,
                    DragAction::ResizeBottom,
                ));
                true
            } else if meta.resize_bottom_left_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    meta.snapped_to,
                    DragAction::ResizeBottomLeft,
                ));
                true
            } else if meta.resize_left_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    meta.snapped_to,
                    DragAction::ResizeLeft,
                ));
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Updates during drag.
    fn update_drag(&mut self, pos: Position) -> bool {
        let Some(drag) = &self.drag else {
            panic!("drag not active")
        };

        let max_x = (self.offset.x + self.area.width).saturating_sub(1);
        let max_y = (self.offset.y + self.area.height).saturating_sub(1);
        let base_area = self.base_area(drag.handle);
        let win_area = self.window_area(drag.handle);

        let (snap, new) = match drag.action {
            DragAction::Move => {
                self.calculate_move(win_area, base_area.as_size(), pos, (max_x, max_y))
            }
            DragAction::ResizeLeft => (None, self.calculate_resize_left(win_area, pos)),
            DragAction::ResizeRight => (None, self.calculate_resize_right(win_area, pos, max_x)),
            DragAction::ResizeBottomLeft => {
                let tmp = self.calculate_resize_left(win_area, pos);
                let tmp = self.calculate_resize_bottom(tmp, pos, max_y);
                (None, tmp)
            }
            DragAction::ResizeBottom => (None, self.calculate_resize_bottom(win_area, pos, max_y)),
            DragAction::ResizeBottomRight => {
                let tmp = self.calculate_resize_right(win_area, pos, max_x);
                let tmp = self.calculate_resize_bottom(tmp, pos, max_y);
                (None, tmp)
            }
        };

        let meta = self.meta.get_mut(&drag.handle).expect("window");
        meta.snapped_to = snap;
        meta.window_area = new;

        true
    }

    /// Finished drag.
    fn commit_drag(&mut self) -> bool {
        let Some(drag) = &self.drag else {
            panic!("drag not active")
        };

        let meta = self.meta.get_mut(&drag.handle).expect("window");
        match drag.action {
            DragAction::Move => {
                if meta.snapped_to.is_none() {
                    meta.base_size = meta.window_area;
                }
            }
            _ => {
                meta.snapped_to = None;
                meta.base_size = meta.window_area;
            }
        }

        self.drag = None;
        true
    }

    /// Cancel drag.
    fn cancel_drag(&mut self) -> bool {
        let Some(drag) = &self.drag else {
            panic!("drag not active")
        };

        let meta = self.meta.get_mut(&drag.handle).expect("window");
        meta.snapped_to = drag.base_snap;
        meta.window_area = meta.base_size;

        self.drag = None;
        true
    }

    // flip maximized state
    fn flip_maximize(&mut self, handle: WinHandle, pos: Position) -> bool {
        if let Some(meta) = self.meta.get_mut(&handle) {
            if meta.move_area.contains(pos) && !self.snap_areas.is_empty() {
                if meta.snapped_to.is_none() {
                    meta.snapped_to = Some(self.snap_areas.len() - 1);
                    meta.window_area = self.snap_areas[self.snap_areas.len() - 1].1;
                } else {
                    meta.snapped_to = None;
                    meta.window_area = meta.base_size;
                }
                self.drag = None;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        match event {
            ct_event!(mouse any for m) if self.mouse.doubleclick(self.area, m) => {
                let pos = Position::new(m.column, m.row);
                if let Some(handle) = self.window_at(pos) {
                    self.flip_maximize(handle, pos).into()
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse down Left for x,y) => {
                let pos = Position::new(*x, *y);
                if let Some(handle) = self.window_at(pos) {
                    // to front
                    let r0 = self.window_to_front(handle).into();
                    // initiate drag
                    let r1 = self.initiate_drag(handle, pos).into();

                    max(r0, r1)
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse drag Left for x,y) => {
                if self.drag.is_some() {
                    self.update_drag(Position::new(*x, *y)).into()
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse up Left for _x,_y) => {
                if self.drag.is_some() {
                    self.commit_drag().into()
                } else {
                    Outcome::Continue
                }
            }
            ct_event!(mouse moved for _x,_y) => {
                // reset on fail otherwise
                if self.drag.is_some() {
                    self.cancel_drag().into()
                } else {
                    Outcome::Continue
                }
            }

            _ => Outcome::Continue,
        }
    }
}
