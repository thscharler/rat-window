use crate::event::WindowsOutcome;
use crate::util::revert_style;
use crate::window_manager::{WindowManager, WindowManagerState};
use crate::{WinFlags, WinHandle, WindowFrame, WindowMode, WindowsState};
use rat_event::util::MouseFlags;
use rat_event::{ct_event, ConsumedEvent, HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer, FocusFlag, HasFocus, Navigation};
use rat_reloc::relocate_area;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Alignment, Position, Rect, Size};
use ratatui::prelude::BlockExt;
use ratatui::style::Style;
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget, WidgetRef};
use std::cmp::max;
use std::collections::HashMap;
use std::mem;
use std::ops::Neg;

/// Deco-One window manager.
#[derive(Debug, Default)]
pub struct DecoOne {
    block: Option<Block<'static>>,
    title_style: Style,
    title_alignment: Alignment,

    focus_style: Option<Style>,
    config_style: Option<Style>,
}

/// Deco-One state.
#[derive(Debug, Default)]
pub struct DecoOneState {
    /// View area in screen coordinates.
    area: Rect,

    /// Render offset. All coordinates are shifted by this
    /// value before rendering.
    offset: Position,
    /// View area in windows coordinates.
    ///
    /// The area starts at offset.x/offset.y and has a size
    /// area.width/area.height.
    area_win: Rect,

    /// Snap to tile areas.
    /// Maps a list of areas in screen coordinates to a
    /// target area in window coordinates.
    snap_areas: Vec<(Vec<Rect>, Rect)>,

    /// Window frame data.
    frames: HashMap<WinHandle, DecoOneFrame>,
    /// Rendering order. Back to front.
    order: Vec<WinHandle>,
    /// Currently dragged mode and window
    drag: Option<Drag>,

    /// Keyboard mode
    mode: WindowMode,
    /// Container focus for all windows.
    container: ContainerFlag,
    /// mouse flags
    mouse: MouseFlags,

    /// Temporary buffer for rendering.
    tmp: Buffer,
}

/// Deco-One window data.
#[derive(Debug)]
struct DecoOneFrame {
    // currently snapped to this snap region.
    snapped_to: Option<u16>,
    // base-line size of the window. in window coordinates.
    base_area_win: Rect,
    // effective window size. in window coordinates.
    area_win: Rect,
    // area for the window content. in window coordinates.
    widget_area_win: Rect,

    // window area in screen coordinates.
    area: Rect,
    // window stacking order as z value
    area_z: u16,

    // close icon. in screen coordinates.
    close_area: Rect,
    // drag to move. in screen coordinates.
    move_area: Rect,
    // drag to resize. in screen coordinates.
    resize_left_area: Rect,
    resize_right_area: Rect,
    resize_bottom_left_area: Rect,
    resize_bottom_area: Rect,
    resize_bottom_right_area: Rect,

    // window container
    container: ContainerFlag,
    // window focus
    focus: FocusFlag,

    // display parameters
    flags: WinFlags,
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
    base_snap: Option<u16>,
    // offset window origin to mouse cursor.
    win_offset: (u16, u16),
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

    /// Config mode style.
    pub fn config_style(mut self, style: Style) -> Self {
        self.config_style = Some(style);
        self
    }
}

impl WindowManager for DecoOne {
    type State = DecoOneState;
    type Outcome = DecoOneOutcome;

    /// Run preparations before rendering any window.
    fn render_init(&self, state: &mut Self::State) {
        let shift = state.shift();

        for (order_idx, handle) in state.order.iter().enumerate() {
            let frame = state.frames.get_mut(handle).expect("window");

            if let Some(idx) = frame.snapped_to {
                frame.area_win = state.snap_areas[idx as usize].1;
            }
            frame.widget_area_win = self.block.inner_if_some(frame.area_win);

            // screen areas for interactions.

            frame.area = relocate_area(frame.area_win, shift, state.area);
            // use a z value of 10xorder. should be enough for
            // possible popups inside a window.
            frame.area_z = order_idx as u16 * 10;

            frame.close_area = if frame.flags.closeable {
                Rect::new(frame.area.right().saturating_sub(4), frame.area.top(), 3, 1)
            } else {
                Rect::default()
            };
            frame.move_area = if frame.flags.moveable {
                Rect::new(frame.area.left(), frame.area.top(), frame.area.width, 1)
            } else {
                Rect::default()
            };
            frame.resize_left_area = if frame.flags.resizable {
                Rect::new(
                    frame.area.left(),
                    frame.area.top() + 1,
                    1,
                    frame.area.height.saturating_sub(2),
                )
            } else {
                Rect::default()
            };
            frame.resize_right_area = if frame.flags.resizable {
                Rect::new(
                    frame.area.right().saturating_sub(1),
                    frame.area.top() + 1,
                    1,
                    frame.area.height.saturating_sub(2),
                )
            } else {
                Rect::default()
            };
            frame.resize_bottom_left_area = if frame.flags.resizable {
                Rect::new(
                    frame.area.left(),
                    frame.area.bottom().saturating_sub(1),
                    1,
                    1,
                )
            } else {
                Rect::default()
            };
            frame.resize_bottom_area = if frame.flags.resizable {
                Rect::new(
                    frame.area.left() + 1,
                    frame.area.bottom().saturating_sub(1),
                    frame.area.width.saturating_sub(2),
                    1,
                )
            } else {
                Rect::default()
            };
            frame.resize_bottom_right_area = if frame.flags.resizable {
                Rect::new(
                    frame.area.right().saturating_sub(1),
                    frame.area.bottom().saturating_sub(1),
                    1,
                    1,
                )
            } else {
                Rect::default()
            };
        }
    }

    /// Get the correctly sized buffer to render the given window.
    fn render_init_buffer(&self, handle: WinHandle, state: &mut Self::State) -> (Rect, Buffer) {
        let frame = state.frames.get(&handle).expect("window");

        let mut tmp = mem::take(&mut state.tmp);
        tmp.resize(frame.area_win);

        (frame.widget_area_win, tmp)
    }

    fn render_window_frame(&self, handle: WinHandle, buf: &mut Buffer, state: &mut Self::State) {
        let frame = state.frames.get(&handle).expect("window");

        let focus = frame.container.get();
        let style = if focus {
            if state.mode == WindowMode::Config {
                self.config_style.unwrap_or(revert_style(self.title_style))
            } else {
                self.focus_style.unwrap_or(revert_style(self.title_style))
            }
        } else {
            self.title_style
        };

        // render border
        self.block.as_ref().render_ref(frame.area_win, buf);

        // complete title bar
        for x in frame.area_win.left() + 1..frame.area_win.right().saturating_sub(1) {
            if let Some(cell) = &mut buf.cell_mut(Position::new(x, frame.area_win.top())) {
                cell.set_style(style);
                cell.set_symbol(" ");
            }
        }

        // title text
        let title_area = Rect::new(
            frame.area_win.left() + 1,
            frame.area_win.top(),
            if frame.flags.closeable {
                frame.close_area.x - (frame.area_win.x + 1)
            } else {
                frame.area_win.width.saturating_sub(2)
            },
            1,
        );
        Text::from(frame.flags.title.as_str())
            .alignment(self.title_alignment)
            .render(title_area, buf);

        if frame.flags.closeable {
            let close_area = Rect::new(
                frame.area_win.right().saturating_sub(4),
                frame.area_win.top(),
                3,
                1,
            );
            Span::from(" \u{2A2F} ").render(close_area, buf);
        }
    }

    fn render_copy_buffer(
        &self,
        buf_win: &mut Buffer,
        screen_area: Rect,
        screen_buf: &mut Buffer,
        state: &mut Self::State,
    ) {
        for (cell_offset, cell) in buf_win.content.drain(..).enumerate() {
            let r_y = cell_offset as u16 / buf_win.area.width;
            let r_x = cell_offset as u16 % buf_win.area.width;

            let tmp_y = buf_win.area.y + r_y;
            let tmp_x = buf_win.area.x + r_x;

            // clip
            if tmp_y < state.offset.y {
                continue;
            }
            if tmp_x < state.offset.x {
                continue;
            }
            if tmp_y - state.offset.y >= screen_area.height {
                continue;
            }
            if tmp_x - state.offset.x >= screen_area.width {
                continue;
            }

            let y = tmp_y - state.offset.y + screen_area.y;
            let x = tmp_x - state.offset.x + screen_area.x;

            if let Some(buf_cell) = screen_buf.cell_mut((x, y)) {
                if cell != Cell::EMPTY {
                    *buf_cell = cell;
                }
            }
        }
    }

    /// Set back the buffer for later reuse.
    fn render_free_buffer(&self, buf: Buffer, state: &mut Self::State) {
        state.tmp = buf;
    }
}

impl Drag {
    /// Drag data for a move.
    fn new_move(handle: WinHandle, snap: Option<u16>, offset: (u16, u16)) -> Self {
        Self {
            action: DragAction::Move,
            handle,
            base_snap: snap,
            win_offset: offset,
        }
    }

    /// Drag data for a resize.
    fn new_resize(handle: WinHandle, snap: Option<u16>, action: DragAction) -> Self {
        Self {
            action,
            handle,
            base_snap: snap,
            win_offset: (0, 0),
        }
    }
}

impl Default for DecoOneFrame {
    fn default() -> Self {
        Self {
            base_area_win: Default::default(),
            snapped_to: None,
            area_win: Default::default(),
            widget_area_win: Default::default(),
            area: Default::default(),
            area_z: 0,
            close_area: Default::default(),
            move_area: Default::default(),
            resize_left_area: Default::default(),
            resize_right_area: Default::default(),
            resize_bottom_left_area: Default::default(),
            resize_bottom_area: Default::default(),
            resize_bottom_right_area: Default::default(),
            container: Default::default(),
            focus: Default::default(),
            flags: Default::default(),
        }
    }
}

impl HasFocus for DecoOneFrame {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(Some(self.container.clone()), self.area, self.area_z);
        builder.add_widget(
            self.focus.clone(),
            self.area,
            self.area_z,
            Navigation::Regular,
        );
        builder.end(tag);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn area_z(&self) -> u16 {
        self.area_z
    }

    fn navigable(&self) -> Navigation {
        Navigation::Regular
    }
}

impl FocusContainer for DecoOneFrame {
    fn build(&self, _builder: &mut FocusBuilder) {}

    fn container(&self) -> Option<ContainerFlag> {
        Some(self.container.clone())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn area_z(&self) -> u16 {
        self.area_z
    }
}

impl WindowFrame for DecoOneFrame {
    fn as_has_focus(&self) -> &dyn HasFocus {
        self
    }

    fn as_focus_container(&self) -> &dyn FocusContainer {
        self
    }
}

impl DecoOneFrame {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    fn named(name: &str) -> Self {
        Self {
            container: ContainerFlag::named(name),
            focus: FocusFlag::named(name),
            ..Default::default()
        }
    }
}

impl DecoOneState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WindowManagerState for DecoOneState {
    /// Current windows area.
    /// In __screen__ coordinates.
    fn area(&self) -> Rect {
        self.area
    }

    /// Change the windows area.
    ///
    /// Recalculates snap areas and snapped window sizes.
    /// Does nothing for regularly placed windows.
    fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.calculate_area_win();
        self.calculate_snaps();
    }

    /// Current offset used for rendering.
    fn offset(&self) -> Position {
        self.offset
    }

    /// Current offset used for rendering.
    fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
        self.calculate_area_win();
        self.calculate_snaps();
    }

    fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;
    }

    fn mode(&self) -> WindowMode {
        self.mode
    }

    fn container(&self) -> ContainerFlag {
        self.container.clone()
    }

    fn window_container(&self, handle: WinHandle) -> ContainerFlag {
        self.frames.get(&handle).expect("window").container.clone()
    }

    fn window_focus(&self, handle: WinHandle) -> FocusFlag {
        self.frames.get(&handle).expect("window").focus.clone()
    }

    fn window_frame(&self, handle: WinHandle) -> &dyn WindowFrame {
        self.frames.get(&handle).expect("window")
    }

    /// Add a new window
    fn insert_window(&mut self, handle: WinHandle) {
        assert!(!self.frames.contains_key(&handle));
        self.frames.insert(
            handle,
            DecoOneFrame::named(format!("{:?}", handle).as_str()),
        );
        self.order.push(handle);
    }

    /// Remove a window.
    fn remove_window(&mut self, handle: WinHandle) {
        self.frames.remove(&handle);
        self.order.retain(|v| *v != handle);
    }

    /// Active window area in window coordinates.
    fn window_area(&self, handle: WinHandle) -> Rect {
        self.frames.get(&handle).expect("window").area_win
    }

    /// Active window area in window coordinates.
    fn set_window_area(&mut self, handle: WinHandle, area: Rect) {
        let frame = self.frames.get_mut(&handle).expect("window");
        frame.area_win = area;
    }

    /// Behaviour flags for a window.
    fn window_flags(&self, handle: WinHandle) -> WinFlags {
        self.frames.get(&handle).expect("window").flags.clone()
    }

    /// Behaviour flags for a window.
    fn set_window_flags(&mut self, handle: WinHandle, flags: WinFlags) {
        self.frames.get_mut(&handle).expect("window").flags = flags;
    }

    /// The window area of the window before being snapped to a region.
    ///
    /// When a widget is being detached from a snap area it
    /// will return to this size.
    ///
    /// When setting a window both [set_window_area] and
    /// [set_base_area] must be called.
    ///
    /// In window coordinates.
    fn window_base_area(&self, handle: WinHandle) -> Rect {
        self.frames.get(&handle).expect("window").base_area_win
    }

    /// The window area of the window before being snapped to a region.
    ///
    /// When a widget is being detached from a snap area it
    /// will return to this size.
    ///
    /// When setting a window both [set_window_area] and
    /// [set_base_area] must be called.
    ///
    /// In window coordinates.
    fn set_window_base_area(&mut self, handle: WinHandle, area: Rect) {
        self.frames.get_mut(&handle).expect("window").base_area_win = area;
    }

    /// The snap-index of the window.
    ///
    /// __Panic__
    /// Panics when the index is out of bounds.
    fn window_snap_idx(&self, handle: WinHandle) -> Option<u16> {
        self.frames.get(&handle).expect("window").snapped_to
    }

    /// The snap-index of the window.
    ///
    /// __Panic__
    /// Panics when the index is out of bounds.
    fn set_window_snap_idx(&mut self, handle: WinHandle, idx: Option<u16>) {
        if let Some(idx) = idx {
            assert!(idx < self.snap_areas.len() as u16);
        }
        self.frames.get_mut(&handle).expect("window").snapped_to = idx;
    }

    /// Area for the window's content.
    ///
    /// In window coordinates.
    fn window_widget_area(&self, handle: WinHandle) -> Rect {
        self.frames.get(&handle).expect("window").widget_area_win
    }

    /// Return a list of the window handles
    /// in rendering order.
    fn handles_render(&self) -> Vec<WinHandle> {
        self.order.clone()
    }

    /// Move the focused window to front.
    fn focus_to_front(&mut self) -> bool {
        // quick check
        if let Some(last) = self.order.last() {
            if let Some(last) = self.frames.get(last) {
                if last.container.get() {
                    return true;
                }
            }
        }

        // iterate and find focused
        let mut new_front = None;
        for (handle, frame) in self.frames.iter() {
            if frame.container.get() {
                new_front = Some(*handle);
                break;
            }
        }
        if let Some(new_front) = new_front {
            self.window_to_front(new_front)
        } else {
            false
        }
    }

    /// Focused window
    fn focused_window(&self) -> Option<WinHandle> {
        for handle in self.order.iter() {
            if let Some(frame) = self.frames.get(handle) {
                if frame.container.get() {
                    return Some(*handle);
                }
            }
        }
        None
    }

    /// Move a window to front.
    #[inline]
    fn window_to_front(&mut self, handle: WinHandle) -> bool {
        if self.order.last() == Some(&handle) {
            false
        } else {
            self.order.retain(|v| *v != handle);
            self.order.push(handle);
            true
        }
    }

    /// Get the front window handle
    fn front_window(&self) -> Option<WinHandle> {
        self.order.last().copied()
    }

    /// Window at the given __screen__ position.
    #[inline]
    fn window_at(&self, pos: Position) -> Option<WinHandle> {
        for handle in self.order.iter().rev().copied() {
            let area = self.frames.get(&handle).expect("window").area;
            if area.contains(pos) {
                return Some(handle);
            }
        }
        None
    }

    fn shift(&self) -> (i16, i16) {
        (
            if self.offset.x < self.area.x {
                (self.area.x - self.offset.x) as i16
            } else {
                (self.offset.x - self.area.x) as i16 * -1i16
            },
            if self.offset.y < self.area.y {
                (self.area.y - self.offset.y) as i16
            } else {
                (self.offset.y - self.area.y) as i16 * -1i16
            },
        )
    }
}

impl DecoOneState {
    /// Calculate window area.
    fn calculate_area_win(&mut self) {
        self.area_win = Rect::new(
            self.offset.x,
            self.offset.y,
            self.area.width,
            self.area.height,
        );
    }

    /// Calculate the snap areas.
    fn calculate_snaps(&mut self) {
        self.snap_areas.clear();

        let area = self.area;
        let area_win = self.area_win;

        let w_clip = area.width / 5;
        let h_clip = area.height / 5;

        // '1': left
        self.snap_areas.push((
            vec![Rect::new(
                area.x + 1,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(area_win.x, area_win.y, area_win.width / 2, area_win.height),
        ));
        // '2': right
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(2),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
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
                area.x + w_clip,
                area.y,
                area.width - 2 * w_clip,
                1,
            )],
            Rect::new(area_win.x, area_win.y, area_win.width, area_win.height / 2),
        ));
        // '4': bottom
        self.snap_areas.push((
            vec![Rect::new(
                area.x + w_clip,
                (area.y + area.height).saturating_sub(1),
                area.width - 2 * w_clip,
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
                Rect::new(area.x, area.y, w_clip, 1),
                Rect::new(area.x, area.y, 1, h_clip),
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
                area.x + 2,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width * 6 / 10,
                area_win.height,
            ),
        ));
        // 'b': same as left
        self.snap_areas.push((
            vec![Rect::new(
                area.x + 1,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width * 5 / 10,
                area_win.height,
            ),
        ));
        // 'c': alt left 2
        self.snap_areas.push((
            vec![Rect::new(
                area.x,
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x,
                area_win.y,
                area_win.width * 4 / 10,
                area_win.height,
            ),
        ));
        // 'd': alt right
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(3),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width * 4 / 10,
                area_win.y,
                area_win.width - area_win.width * 4 / 10,
                area_win.height,
            ),
        ));
        // 'e': same as right
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(2),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width * 5 / 10,
                area_win.y,
                area_win.width - area_win.width * 5 / 10,
                area_win.height,
            ),
        ));
        // 'f': alt right 2
        self.snap_areas.push((
            vec![Rect::new(
                (area.x + area.width).saturating_sub(1),
                area.y + h_clip,
                1,
                area.height - 2 * h_clip,
            )],
            Rect::new(
                area_win.x + area_win.width * 6 / 10,
                area_win.y,
                area_win.width - area_win.width * 6 / 10,
                area_win.height,
            ),
        ));
        // 'g' or '0'==last: full area
        self.snap_areas.push((Vec::default(), self.area_win));
    }

    /// Calculate the new window area when resizing the left side.
    fn calculate_resize_left(&self, mut area: Rect, pos: Position) -> Rect {
        let right = area.x + area.width;
        let shift = self.shift();
        area.x = pos.x.saturating_add_signed(-shift.0);
        if area.x < self.area_win.x {
            area.x = self.area_win.x;
        } else if area.x >= right.saturating_sub(2) {
            area.x = right.saturating_sub(2);
        }
        area.width = right.saturating_sub(area.x);
        area
    }

    /// Calculate the new window area when resizing the right side.
    fn calculate_resize_right(&self, mut area: Rect, pos: Position, max_x: u16) -> Rect {
        let shift = self.shift();
        let pos_x = pos.x.saturating_add_signed(-shift.0);

        area.width = pos_x.saturating_sub(area.x);
        if area.width < 2 {
            area.width = 2;
        }
        if area.x + area.width >= max_x {
            area.width = max_x.saturating_sub(area.x) + 1;
        }
        area
    }

    /// Calculate the new window size when resizing the bottom side.
    fn calculate_resize_bottom(&self, mut area: Rect, pos: Position, max_y: u16) -> Rect {
        let shift = self.shift();
        let pos_y = pos.y.saturating_add_signed(-shift.1);

        area.height = pos_y.saturating_sub(area.y);
        if area.height < 2 {
            area.height = 2;
        }
        if area.y + area.height >= max_y {
            area.height = max_y.saturating_sub(area.y) + 1;
        }
        area
    }

    /// Calculate the new window when moving.
    /// This handles the snap areas too.
    fn calculate_move(
        &self,
        mut win_area: Rect,
        base_size: Size,
        pos: Position,
        max: (u16, u16),
    ) -> (Option<u16>, Rect) {
        // match a snap area?
        for (idx, (snap_area, resize_to)) in self.snap_areas.iter().enumerate() {
            if snap_area.iter().find(|v| v.contains(pos)).is_some() {
                return (Some(idx as u16), *resize_to);
            }
        }

        let Some(drag) = &self.drag else {
            panic!("drag not active")
        };

        // regular move
        let shift = self.shift();
        win_area.x = pos
            .x
            .saturating_sub(drag.win_offset.0)
            .saturating_add_signed(-shift.0);
        win_area.y = pos
            .y
            .saturating_sub(drag.win_offset.1)
            .saturating_add_signed(-shift.1);
        win_area.width = base_size.width;
        win_area.height = base_size.height;

        if win_area.y < self.area_win.y {
            win_area.y = self.area_win.y;
        } else if win_area.y >= max.1 {
            win_area.y = max.1;
        }
        if win_area.x + win_area.width < self.area_win.x {
            win_area.x = self.area_win.x.saturating_sub(win_area.width);
        }
        if win_area.x >= max.0 {
            win_area.x = max.0;
        }
        (None, win_area)
    }
}

impl DecoOneState {
    /// Start dragging.
    fn initiate_drag(&mut self, handle: WinHandle, pos: Position) -> DecoOneOutcome {
        if let Some(frame) = self.frames.get(&handle) {
            if frame.move_area.contains(pos) {
                self.drag = Some(Drag::new_move(
                    handle,
                    frame.snapped_to,
                    if frame.area_win.as_size() != frame.base_area_win.as_size() {
                        (0, 0).into()
                    } else {
                        (pos.x - frame.move_area.x, pos.y - frame.move_area.y).into()
                    },
                ));
                DecoOneOutcome::Moving(handle)
            } else if frame.resize_right_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    frame.snapped_to,
                    DragAction::ResizeRight,
                ));
                DecoOneOutcome::Resizing(handle)
            } else if frame.resize_bottom_right_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    frame.snapped_to,
                    DragAction::ResizeBottomRight,
                ));
                DecoOneOutcome::Resizing(handle)
            } else if frame.resize_bottom_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    frame.snapped_to,
                    DragAction::ResizeBottom,
                ));
                DecoOneOutcome::Resizing(handle)
            } else if frame.resize_bottom_left_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    frame.snapped_to,
                    DragAction::ResizeBottomLeft,
                ));
                DecoOneOutcome::Resizing(handle)
            } else if frame.resize_left_area.contains(pos) {
                self.drag = Some(Drag::new_resize(
                    handle,
                    frame.snapped_to,
                    DragAction::ResizeLeft,
                ));
                DecoOneOutcome::Resizing(handle)
            } else {
                DecoOneOutcome::Continue
            }
        } else {
            DecoOneOutcome::Continue
        }
    }

    /// Updates during drag.
    #[inline]
    fn update_drag(&mut self, pos: Position) -> DecoOneOutcome {
        let Some(drag) = &self.drag else {
            return DecoOneOutcome::Continue;
        };

        let max_x = (self.area_win.x + self.area_win.width).saturating_sub(1);
        let max_y = (self.area_win.y + self.area_win.height).saturating_sub(1);
        let base_area = self.window_base_area(drag.handle);
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

        let frame = self.frames.get_mut(&drag.handle).expect("window");
        frame.snapped_to = snap;
        frame.area_win = new;

        match drag.action {
            DragAction::Move => DecoOneOutcome::Moving(drag.handle),
            DragAction::ResizeLeft
            | DragAction::ResizeRight
            | DragAction::ResizeBottomLeft
            | DragAction::ResizeBottom
            | DragAction::ResizeBottomRight => DecoOneOutcome::Resizing(drag.handle),
        }
    }

    /// Finished drag.
    #[inline]
    fn commit_drag(&mut self) -> DecoOneOutcome {
        let Some(drag) = &self.drag else {
            return DecoOneOutcome::Continue;
        };

        let frame = self.frames.get_mut(&drag.handle).expect("window");

        let handle = drag.handle;
        let action = drag.action;
        self.drag = None;

        match action {
            DragAction::Move => {
                if frame.snapped_to.is_none() {
                    frame.base_area_win = frame.area_win;
                }
                DecoOneOutcome::Moved(handle)
            }
            _ => {
                frame.snapped_to = None;
                frame.base_area_win = frame.area_win;
                DecoOneOutcome::Resized(handle)
            }
        }
    }

    /// Cancel drag.
    #[inline]
    fn cancel_drag(&mut self) -> DecoOneOutcome {
        let Some(drag) = &self.drag else {
            return DecoOneOutcome::Continue;
        };

        let frame = self.frames.get_mut(&drag.handle).expect("window");
        frame.snapped_to = drag.base_snap;
        frame.area_win = frame.base_area_win;

        let handle = drag.handle;
        let action = drag.action;
        self.drag = None;

        match action {
            DragAction::Move => DecoOneOutcome::Moved(handle),
            DragAction::ResizeLeft
            | DragAction::ResizeRight
            | DragAction::ResizeBottomLeft
            | DragAction::ResizeBottom
            | DragAction::ResizeBottomRight => DecoOneOutcome::Resized(handle),
        }
    }

    /// Flip maximized state.
    #[inline]
    fn flip_maximize(&mut self, handle: WinHandle, pos: Position) -> DecoOneOutcome {
        if let Some(frame) = self.frames.get_mut(&handle) {
            if frame.move_area.contains(pos) && !self.snap_areas.is_empty() {
                self.snap_to(handle, self.snap_areas.len().saturating_sub(1) as u16)
            } else {
                DecoOneOutcome::Continue
            }
        } else {
            DecoOneOutcome::Continue
        }
    }

    /// Snap a window to the given area.
    fn snap_to(&mut self, handle: WinHandle, snap_idx: u16) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };

        if snap_idx < self.snap_areas.len() as u16 {
            if frame.snapped_to == Some(snap_idx) {
                frame.snapped_to = None;
                frame.area_win = frame.base_area_win;
            } else {
                frame.snapped_to = Some(snap_idx);
                frame.area_win = self.snap_areas[snap_idx as usize].1;
            }
            DecoOneOutcome::Snap(handle, snap_idx)
        } else {
            DecoOneOutcome::Continue
        }
    }

    fn move_up(&mut self, handle: WinHandle) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.y = frame.area_win.y.saturating_sub(1);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Moved(handle)
    }

    fn move_down(&mut self, handle: WinHandle) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.y = frame.area_win.y.saturating_add(1);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Moved(handle)
    }

    fn move_left(&mut self, handle: WinHandle) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.x = frame.area_win.x.saturating_sub(1);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Moved(handle)
    }

    fn move_right(&mut self, handle: WinHandle) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.x = frame.area_win.x.saturating_add(1);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Moved(handle)
    }

    fn resize_top(&mut self, handle: WinHandle, by: i16) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.y = frame.area_win.y.saturating_add_signed(by.neg());
        frame.area_win.height = frame.area_win.height.saturating_add_signed(by);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Resized(handle)
    }

    fn resize_bottom(&mut self, handle: WinHandle, by: i16) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.height = frame.area_win.height.saturating_add_signed(by);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Resized(handle)
    }

    fn resize_left(&mut self, handle: WinHandle, by: i16) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.x = frame.area_win.x.saturating_add_signed(by.neg());
        frame.area_win.width = frame.area_win.width.saturating_add_signed(by);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Resized(handle)
    }

    fn resize_right(&mut self, handle: WinHandle, by: i16) -> DecoOneOutcome {
        let Some(frame) = self.frames.get_mut(&handle) else {
            panic!("invalid handle");
        };
        frame.snapped_to = None;
        frame.area_win.width = frame.area_win.width.saturating_add_signed(by);
        frame.base_area_win = frame.area_win;
        DecoOneOutcome::Resized(handle)
    }
}

/// Result of event handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecoOneOutcome {
    /// The given event has not been used at all.
    Continue,
    /// The event has been recognized, but the result was nil.
    /// Further processing for this event may stop.
    Unchanged,
    /// The event has been recognized and there is some change
    /// due to it.
    /// Further processing for this event may stop.
    /// Rendering the ui is advised.
    Changed,
    /// Currently moving.
    Moving(WinHandle),
    /// Currently resizing.
    Resizing(WinHandle),
    /// Snap to a region occurred.
    Snap(WinHandle, u16),
    /// Moved to front, old front window.
    ToFront(WinHandle, Option<WinHandle>),
    /// Moved
    Moved(WinHandle),
    /// Resized
    Resized(WinHandle),
}

impl ConsumedEvent for DecoOneOutcome {
    fn is_consumed(&self) -> bool {
        *self != DecoOneOutcome::Continue
    }
}

// Useful for converting most navigation/edit results.
impl From<bool> for DecoOneOutcome {
    fn from(value: bool) -> Self {
        if value {
            DecoOneOutcome::Changed
        } else {
            DecoOneOutcome::Unchanged
        }
    }
}

impl From<DecoOneOutcome> for Outcome {
    fn from(value: DecoOneOutcome) -> Self {
        match value {
            DecoOneOutcome::Continue => Outcome::Continue,
            DecoOneOutcome::Unchanged => Outcome::Unchanged,
            DecoOneOutcome::Changed => Outcome::Changed,
            DecoOneOutcome::Snap(_, _) => Outcome::Changed,
            DecoOneOutcome::ToFront(_, _) => Outcome::Changed,
            DecoOneOutcome::Moving(_) => Outcome::Changed,
            DecoOneOutcome::Moved(_) => Outcome::Changed,
            DecoOneOutcome::Resizing(_) => Outcome::Changed,
            DecoOneOutcome::Resized(_) => Outcome::Changed,
        }
    }
}

impl From<DecoOneOutcome> for WindowsOutcome {
    fn from(value: DecoOneOutcome) -> Self {
        match value {
            DecoOneOutcome::Continue => WindowsOutcome::Continue,
            DecoOneOutcome::Unchanged => WindowsOutcome::Unchanged,
            DecoOneOutcome::Changed => WindowsOutcome::Changed,
            DecoOneOutcome::Snap(h, i) => WindowsOutcome::Snap(h, i),
            DecoOneOutcome::ToFront(h, oh) => WindowsOutcome::ToFront(h, oh),
            DecoOneOutcome::Moving(h) => WindowsOutcome::Moving(h),
            DecoOneOutcome::Moved(h) => WindowsOutcome::Moved(h),
            DecoOneOutcome::Resizing(h) => WindowsOutcome::Resizing(h),
            DecoOneOutcome::Resized(h) => WindowsOutcome::Resized(h),
        }
    }
}

impl HandleEvent<crossterm::event::Event, Regular, DecoOneOutcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> DecoOneOutcome {
        let mut r = DecoOneOutcome::Continue;

        if self.mode == WindowMode::Config && self.container.get() {
            if let Some(handle) = self.focused_window() {
                r = r.or_else(|| match event {
                    ct_event!(key press '0') => {
                        self.snap_to(handle, self.snap_areas.len().saturating_sub(1) as u16)
                    }
                    ct_event!(key press f@'1'..='9') => {
                        let snap_idx = *f as usize - '1' as usize;
                        self.snap_to(handle, snap_idx as u16)
                    }
                    ct_event!(key press f@'a'..='z') => {
                        let snap_idx = (*f as usize - 'a' as usize) + 9;
                        self.snap_to(handle, snap_idx as u16)
                    }

                    ct_event!(keycode press Up) => self.move_up(handle),
                    ct_event!(keycode press CONTROL_SHIFT-Up) => self.resize_top(handle, 1),
                    ct_event!(keycode press CONTROL_SHIFT-Down) => self.resize_top(handle, -1),

                    ct_event!(keycode press Down) => self.move_down(handle),
                    ct_event!(keycode press CONTROL-Down) => self.resize_bottom(handle, 1),
                    ct_event!(keycode press CONTROL-Up) => self.resize_bottom(handle, -1),

                    ct_event!(keycode press Left) => self.move_left(handle),
                    ct_event!(keycode press CONTROL_SHIFT-Left) => self.resize_left(handle, 1),
                    ct_event!(keycode press CONTROL_SHIFT-Right) => self.resize_left(handle, -1),
                    ct_event!(keycode press Right) => self.move_right(handle),
                    ct_event!(keycode press CONTROL-Left) => self.resize_right(handle, -1),
                    ct_event!(keycode press CONTROL-Right) => self.resize_right(handle, 1),

                    _ => DecoOneOutcome::Continue,
                });
            }
        }

        r.or_else(|| self.handle(event, MouseOnly))
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, DecoOneOutcome> for DecoOneState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> DecoOneOutcome {
        let mut r = DecoOneOutcome::Continue;

        r = r.or_else(|| match event {
            ct_event!(mouse any for m) if self.mouse.doubleclick(self.area, m) => {
                let pos = Position::new(m.column, m.row);
                if let Some(handle) = self.window_at(pos) {
                    self.cancel_drag();
                    self.flip_maximize(handle, pos)
                } else {
                    DecoOneOutcome::Continue
                }
            }
            ct_event!(mouse down Left for x,y) => {
                let pos = Position::new(*x, *y);
                if let Some(handle) = self.window_at(pos) {
                    let old_handle = self.front_window();
                    let r0 = if self.window_to_front(handle) {
                        DecoOneOutcome::ToFront(handle, old_handle)
                    } else {
                        DecoOneOutcome::Continue
                    };
                    let r1 = self.initiate_drag(handle, pos).into();
                    max(r0, r1)
                } else {
                    DecoOneOutcome::Continue
                }
            }
            ct_event!(mouse drag Left for x,y) => self.update_drag(Position::new(*x, *y)).into(),
            ct_event!(mouse up Left for _x,_y) => self.commit_drag().into(),
            ct_event!(mouse moved for _x,_y) => self.cancel_drag().into(), // reset drag on unknown
            _ => DecoOneOutcome::Continue,
        });
        r
    }
}

impl<T, S> WindowsState<T, S, DecoOne>
where
    T: ?Sized,
    S: ?Sized,
{
}
