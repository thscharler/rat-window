use crate::util::{copy_buffer, revert_style};
use crate::win_flags::WinFlags;
use crate::windows::WinHandle;
use rat_event::util::MouseFlags;
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{FocusFlag, HasFocus};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Position, Rect, Size};
use ratatui::prelude::{BlockExt, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget, WidgetRef};
use std::cmp::max;
use std::collections::HashMap;
use std::mem;
use std::ops::Neg;

///
/// Deco-One window manager.
///
#[derive(Debug, Default)]
pub struct DecoOne {
    block: Option<Block<'static>>,
    title_style: Style,
    title_alignment: Alignment,

    focus_style: Option<Style>,
    meta_style: Option<Style>,
}

///
/// State for Deco-One.
///
#[derive(Debug, Default)]
pub struct DecoOneState {
    /// View area in screen coordinates.
    area: Rect,
    /// View area in windows coordinates.
    area_win: Rect,

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

    /// Keyboard mode
    mode: KeyboardMode,
    /// Windows has the focus?
    focus: FocusFlag,
    /// mouse flags
    mouse: MouseFlags,

    /// Temporary buffer for rendering.
    tmp: Buffer,
}

/// Current keyboard mode.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
enum KeyboardMode {
    /// Regular behaviour
    #[default]
    Regular,
    /// Do work on the windows themselves.
    Meta,
}

/// Current drag action.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DragAction {
    Move,
    ResizeLeft,
    ResizeRight,
    ResizeBottomLeft,
    ResizeBottom,
    ResizeBottomRight,
}

/// Current drag data.
#[derive(Debug, Clone)]
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
    /// Drag data for a move.
    fn new_move(handle: WinHandle, snap: Option<usize>, offset: (u16, u16)) -> Self {
        Self {
            action: DragAction::Move,
            handle,
            base_snap: snap,
            win_offset: offset,
        }
    }

    /// Drag data for a resize.
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
    /// Create window manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Block for the window.
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Title style for the window.
    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    /// Title alignment.
    pub fn title_alignment(mut self, align: Alignment) -> Self {
        self.title_alignment = align;
        self
    }

    /// Focus style.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }

    /// Meta style.
    pub fn meta_style(mut self, style: Style) -> Self {
        self.meta_style = Some(style);
        self
    }
}

impl DecoOne {
    /// Window manager operation.
    ///
    /// Calculate areas and flags for the given window.
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

    /// Get the correctly sized buffer to render the given window.
    pub fn get_buffer(&self, handle: WinHandle, state: &mut DecoOneState) -> Buffer {
        let mut tmp = mem::take(&mut state.tmp);
        tmp.resize(state.window_area(handle));
        tmp
    }

    /// Set back the buffer for later reuse.
    pub fn set_buffer(&self, tmp: Buffer, state: &mut DecoOneState) {
        state.tmp = tmp;
    }

    /// Render the window decorations.
    pub fn render_window(&mut self, handle: WinHandle, tmp: &mut Buffer, state: &mut DecoOneState) {
        let meta = state.meta.get(&handle).expect("window");

        let focus = meta.flags.focus.get();
        let style = if focus {
            if state.mode == KeyboardMode::Meta {
                self.meta_style.unwrap_or(revert_style(self.title_style))
            } else {
                self.focus_style.unwrap_or(revert_style(self.title_style))
            }
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

    /// Copy the temporary buffer to screen.
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

    /// Get the focus flag for [Windows]
    pub fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    /// Current windows area.
    /// In __screen__ coordinates.
    pub fn area(&self) -> Rect {
        self.area
    }

    /// Change the windows area.
    ///
    /// Recalculates snap areas and snapped window sizes.
    /// Does nothing for regularly placed windows.
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.area_win = Rect::from((
            self.screen_to_win(area.as_position()).expect("area"),
            area.as_size(),
        ));

        self.calculate_snaps();
        self.update_snapped_windows();
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

    /// Focus the top window
    pub fn focus_last_window(&mut self) -> bool {
        for meta in self.meta.values_mut() {
            meta.flags.focus.clear();
        }
        if let Some(handle) = self.order.last() {
            self.meta
                .get(&handle)
                .expect("window")
                .flags
                .focus
                .set(true);
        }
        true
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

        let area_win = self.area_win;

        let w_clip = area_win.width / 5;
        let h_clip = area_win.height / 5;

        // '1': left
        self.snap_areas.push((
            vec![Rect::new(
                area_win.x + 1,
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(area_win.x, area_win.y, area_win.width / 2, area_win.height),
        ));
        // '2': right
        self.snap_areas.push((
            vec![Rect::new(
                (area_win.x + area_win.width).saturating_sub(2),
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width / 2,
                area_win.y,
                area_win.width - area_win.width / 2,
                area_win.height,
            ),
        ));
        // '3': top
        self.snap_areas.push((
            vec![Rect::new(
                area_win.x + w_clip,
                area_win.y,
                area_win.width - 2 * w_clip,
                1,
            )],
            Rect::new(area_win.x, area_win.y, area_win.width, area_win.height / 2),
        ));
        // '4': bottom
        self.snap_areas.push((
            vec![Rect::new(
                area_win.x + w_clip,
                (area_win.y + area_win.height).saturating_sub(1),
                area_win.width - 2 * w_clip,
                1,
            )],
            Rect::new(
                area_win.x,
                area_win.y + area_win.height / 2,
                area_win.width,
                area_win.height - area_win.height / 2,
            ),
        ));
        // '5': top left
        self.snap_areas.push((
            vec![
                Rect::new(area_win.x, area_win.y, w_clip, 1),
                Rect::new(area_win.x, area_win.y, 1, h_clip),
            ],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width / 2,
                area_win.height / 2,
            ),
        ));
        // '6': top right
        self.snap_areas.push((
            vec![
                Rect::new(
                    (area_win.x + area_win.width - w_clip).saturating_sub(1),
                    area_win.y,
                    w_clip,
                    1,
                ),
                Rect::new(
                    (area_win.x + area_win.width).saturating_sub(1), //
                    area_win.y,
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area_win.x + area_win.width / 2,
                area_win.y,
                area_win.width - area_win.width / 2,
                area_win.height / 2,
            ),
        ));
        // '7: bottom left
        self.snap_areas.push((
            vec![
                Rect::new(
                    area_win.x, //
                    (area_win.y + area_win.height).saturating_sub(1),
                    w_clip,
                    1,
                ),
                Rect::new(
                    area_win.x,
                    (area_win.y + area_win.height - h_clip).saturating_sub(1),
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area_win.x,
                area_win.y + area_win.height / 2,
                area_win.width / 2,
                area_win.height - area_win.height / 2,
            ),
        ));
        // '8': bottom right
        self.snap_areas.push((
            vec![
                Rect::new(
                    (area_win.x + area_win.width - w_clip).saturating_sub(1),
                    (area_win.y + area_win.height).saturating_sub(1),
                    w_clip,
                    1,
                ),
                Rect::new(
                    (area_win.x + area_win.width).saturating_sub(1),
                    (area_win.y + area_win.height - h_clip).saturating_sub(1),
                    1,
                    h_clip,
                ),
            ],
            Rect::new(
                area_win.x + area_win.width / 2,
                area_win.y + area_win.height / 2,
                area_win.width - area_win.width / 2,
                area_win.height - area_win.height / 2,
            ),
        ));

        // '9' ... empty
        self.snap_areas.push((Vec::default(), Rect::default()));

        // 'a': alt left
        self.snap_areas.push((
            vec![Rect::new(
                area_win.x + 2,
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width * 6 / 10,
                area_win.height,
            ),
        ));
        // 'b': alt left 2
        self.snap_areas.push((
            vec![Rect::new(
                area_win.x,
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width * 4 / 10,
                area_win.height,
            ),
        ));
        // 'c': alt right
        self.snap_areas.push((
            vec![Rect::new(
                (area_win.x + area_win.width).saturating_sub(3),
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width * 4 / 10,
                area_win.y,
                area_win.width - area_win.width * 4 / 10,
                area_win.height,
            ),
        ));
        // 'd': alt right 2
        self.snap_areas.push((
            vec![Rect::new(
                (area_win.x + area_win.width).saturating_sub(1),
                area_win.y + h_clip,
                1,
                area_win.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width * 6 / 10,
                area_win.y,
                area_win.width - area_win.width * 6 / 10,
                area_win.height,
            ),
        ));
        // 'e' or '0'==last: full area
        self.snap_areas.push((Vec::default(), self.area_win));
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
    fn update_snapped_windows(&mut self) {
        for meta in self.meta.values_mut() {
            if let Some(idx) = meta.snapped_to {
                meta.window_area = self.snap_areas[idx].1;
            }
        }
    }

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

        let max_x = (self.offset.x + self.area_win.width).saturating_sub(1);
        let max_y = (self.offset.y + self.area_win.height).saturating_sub(1);
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

    fn snap_to(&mut self, handle: WinHandle, snap_idx: usize) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };

        if snap_idx < self.snap_areas.len() {
            if meta.snapped_to == Some(snap_idx) {
                meta.snapped_to = None;
                meta.window_area = meta.base_size;
            } else {
                meta.snapped_to = Some(snap_idx);
                meta.window_area = self.snap_areas[snap_idx].1;
            }
            true
        } else {
            false
        }
    }

    fn focus_next(&mut self) -> bool {
        let mut focus_idx = 0;
        for (idx, handle) in self.order.iter().enumerate() {
            if self.is_window_focused(*handle) {
                focus_idx = idx;
                break;
            }
        }
        focus_idx += 1;
        if focus_idx >= self.order.len() {
            focus_idx = 0;
        }
        if let Some(handle) = self.order.get(focus_idx).copied() {
            self.set_focused_window(handle);
            self.window_to_front(handle);
            true
        } else {
            false
        }
    }

    fn focus_prev(&mut self) -> bool {
        let mut focus_idx = 0;
        for (idx, handle) in self.order.iter().enumerate() {
            if self.is_window_focused(*handle) {
                focus_idx = idx;
                break;
            }
        }
        if focus_idx > 0 {
            focus_idx -= 1;
        } else {
            focus_idx = self.order.len().saturating_sub(1);
        }
        if let Some(handle) = self.order.get(focus_idx).copied() {
            self.set_focused_window(handle);
            self.window_to_front(handle);
            true
        } else {
            false
        }
    }

    fn move_up(&mut self, handle: WinHandle) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.y = meta.window_area.y.saturating_sub(1);
        meta.base_size = meta.window_area;
        true
    }

    fn move_down(&mut self, handle: WinHandle) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.y = meta.window_area.y.saturating_add(1);
        meta.base_size = meta.window_area;
        true
    }

    fn move_left(&mut self, handle: WinHandle) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.x = meta.window_area.x.saturating_sub(1);
        meta.base_size = meta.window_area;
        true
    }

    fn move_right(&mut self, handle: WinHandle) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.x = meta.window_area.x.saturating_add(1);
        meta.base_size = meta.window_area;
        true
    }

    fn resize_top(&mut self, handle: WinHandle, by: i16) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.y = meta.window_area.y.saturating_add_signed(by.neg());
        meta.window_area.height = meta.window_area.height.saturating_add_signed(by);
        meta.base_size = meta.window_area;
        true
    }

    fn resize_bottom(&mut self, handle: WinHandle, by: i16) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.height = meta.window_area.height.saturating_add_signed(by);
        meta.base_size = meta.window_area;
        true
    }

    fn resize_left(&mut self, handle: WinHandle, by: i16) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.x = meta.window_area.x.saturating_add_signed(by.neg());
        meta.window_area.width = meta.window_area.width.saturating_add_signed(by);
        meta.base_size = meta.window_area;
        true
    }

    fn resize_right(&mut self, handle: WinHandle, by: i16) -> bool {
        let Some(meta) = self.meta.get_mut(&handle) else {
            panic!("invalid handle");
        };
        meta.snapped_to = None;
        meta.window_area.width = meta.window_area.width.saturating_add_signed(by);
        meta.base_size = meta.window_area;
        true
    }
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let mut r = Outcome::Continue;

        if self.focus.is_focused() {
            r = r.or_else(|| match event {
                ct_event!(keycode press F(8)) => {
                    self.mode = match self.mode {
                        KeyboardMode::Regular => KeyboardMode::Meta,
                        KeyboardMode::Meta => KeyboardMode::Regular,
                    };
                    Outcome::Changed
                }
                _ => Outcome::Continue,
            });
        }
        if self.mode == KeyboardMode::Meta && self.focus.is_focused() {
            if self.focused_window().is_none() {
                self.focus_last_window();
            }
            if let Some(handle) = self.focused_window() {
                r = r.or_else(|| match event {
                    ct_event!(keycode press Tab) => self.focus_next().into(),
                    ct_event!(keycode press SHIFT-Tab) => self.focus_prev().into(),
                    ct_event!(key press '0') => self
                        .snap_to(handle, self.snap_areas.len().saturating_sub(1))
                        .into(),
                    ct_event!(key press f@'1'..='9') => {
                        let snap_idx = *f as usize - '1' as usize;
                        self.snap_to(handle, snap_idx).into()
                    }
                    ct_event!(key press f@'a'..='z') => {
                        let snap_idx = (*f as usize - 'a' as usize) + 9;
                        self.snap_to(handle, snap_idx).into()
                    }

                    ct_event!(keycode press Up) => self.move_up(handle).into(),
                    ct_event!(keycode press CONTROL_SHIFT-Up) => self.resize_top(handle, 1).into(),
                    ct_event!(keycode press CONTROL_SHIFT-Down) => {
                        self.resize_top(handle, -1).into()
                    }

                    ct_event!(keycode press Down) => self.move_down(handle).into(),
                    ct_event!(keycode press CONTROL-Down) => self.resize_bottom(handle, 1).into(),
                    ct_event!(keycode press CONTROL-Up) => self.resize_bottom(handle, -1).into(),

                    ct_event!(keycode press Left) => self.move_left(handle).into(),
                    ct_event!(keycode press CONTROL_SHIFT-Left) => {
                        self.resize_left(handle, 1).into()
                    }
                    ct_event!(keycode press CONTROL_SHIFT-Right) => {
                        self.resize_left(handle, -1).into()
                    }
                    ct_event!(keycode press Right) => self.move_right(handle).into(),
                    ct_event!(keycode press CONTROL-Left) => self.resize_right(handle, -1).into(),
                    ct_event!(keycode press CONTROL-Right) => self.resize_right(handle, 1).into(),

                    _ => Outcome::Continue,
                });
            }
        }

        r = r.or_else(|| match event {
            ct_event!(mouse any for m) if self.mouse.doubleclick(self.area_win, m) => {
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
                    // focus window
                    let r1 = self.set_focused_window(handle).into();
                    // initiate drag
                    let r2 = self.initiate_drag(handle, pos).into();

                    max(max(r0, r1), r2)
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
        });
        r
    }
}
