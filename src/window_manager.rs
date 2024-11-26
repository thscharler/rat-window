use crate::{WinBaseState, WinFlags, WinHandle, Windows, WindowsState};
use rat_focus::FocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use std::borrow::Cow;

pub trait WindowManager {
    type State: WindowManagerState;

    /// Initialize rendering of the given window.
    fn render_init_window(&self, handle: WinHandle, flags: WinFlags, state: &mut Self::State);

    /// Create the buffer to render the given window.
    ///
    /// Returns the buffer and the area where the
    /// window content can be rendered.
    fn render_init_buffer(&self, handle: WinHandle, state: &mut Self::State) -> (Rect, Buffer);

    /// Render the window frame.
    fn render_window_frame(&self, handle: WinHandle, buf: &mut Buffer, state: &mut Self::State);

    /// Copy the window buffer to screen.
    ///
    /// * screen_area: The full area for the Windows widget.
    /// * screen_buf: The target buffer.
    fn render_copy_buffer(
        &self,
        buf: &mut Buffer,
        screen_area: Rect,
        screen_buf: &mut Buffer,
        state: &mut Self::State,
    );

    /// Frees/reuses the buffer after rendering a window is finished.
    fn render_free_buffer(&self, buf: Buffer, state: &mut Self::State);
}

pub trait WindowManagerState {
    /// The [Windows] area in __screen__ coordinates.
    fn area(&self) -> Rect;

    /// The [Windows] area in __screen__ coordinates.
    fn set_area(&mut self, area: Rect);

    /// The offset of the top-left corner of the
    /// Windows area in windows-coordinates.
    ///
    /// Setting the offset allows windows to move left/top
    /// outside the area.
    fn offset(&self) -> Position;

    /// The offset of the top-left corner of the
    /// Windows area in windows-coordinates.
    ///
    /// Setting the offset allows windows to move left/top
    /// outside the area.
    fn set_offset(&mut self, offset: Position);

    /// Focus flag for the [Windows] widget.
    fn focus(&self) -> FocusFlag;

    /// Insert a window into the window manager.
    fn insert_window(&mut self, handle: WinHandle);

    /// Remove a window from the window manager.
    fn remove_window(&mut self, handle: WinHandle);

    /// The active window area including the frame.
    fn window_area(&self, handle: WinHandle) -> Rect;

    /// The active window area including the frame.
    fn set_window_area(&mut self, handle: WinHandle, area: Rect);

    /// The window area of the window before being snapped to a region.
    ///
    /// When a widget is being detached from a snap area it
    /// will return to this size.
    ///
    /// When setting a window both [set_window_area] and
    /// [set_base_area] must be called.
    fn window_base_area(&self, handle: WinHandle) -> Rect;

    /// The window area of the window before being snapped to a region.
    ///
    /// When a widget is being detached from a snap area it
    /// will return to this size.
    ///
    /// When setting a window both [set_window_area] and
    /// [set_base_area] must be called.
    fn set_window_base_area(&mut self, handle: WinHandle, area: Rect);

    /// The snap-index of the window.
    ///
    /// __Panic__
    /// Panics when the index is out of bounds.
    fn window_snap_idx(&self, handle: WinHandle) -> Option<usize>;

    /// The snap-index of the window.
    ///
    /// __Panic__
    /// Panics when the index is out of bounds.
    fn set_window_snap_idx(&mut self, handle: WinHandle, idx: Option<usize>);

    /// Area for the window's content.
    fn window_widget_area(&self, handle: WinHandle) -> Rect;

    /// Is the window focused?
    fn is_focused_window(&self, handle: WinHandle) -> bool;

    /// Handle of the focused window.
    fn focused_window(&self) -> Option<WinHandle>;

    /// Focus the top window.
    fn focus_top_window(&mut self) -> bool;

    /// Focus the given window.
    fn focus_window(&mut self, handle: WinHandle) -> bool;

    /// Return a list of the window handles
    /// in rendering order.
    fn handles(&self) -> Vec<WinHandle>;

    /// Move a window to front.
    fn window_to_front(&mut self, handle: WinHandle) -> bool;

    /// Window at the given __screen__ position.
    fn window_at(&self, pos: Position) -> Option<WinHandle>;

    /// Add the offset to the given area.
    /// This is useful when you create new windows and don't
    /// want to have them outside the visible area anyway.
    fn add_offset(&self, area: Rect) -> Rect;

    /// Translate screen coordinates to window coordinates.
    fn screen_to_win(&self, pos: Position) -> Option<Position>;

    /// Translate window coordinates to screen coordinates
    fn win_to_screen(&self, pos: Position) -> Option<Position>;
}

/// Relocate mouse events to window coordinates.
pub fn relocate_event<'a, 'b>(
    window_manager: &'a impl WindowManagerState,
    event: &'b crossterm::event::Event,
) -> Option<Cow<'b, crossterm::event::Event>> {
    match event {
        crossterm::event::Event::Mouse(mouse_event) => {
            if let Some(pos) =
                window_manager.screen_to_win(Position::new(mouse_event.column, mouse_event.row))
            {
                let mut mm = mouse_event.clone();
                mm.column = pos.x;
                mm.row = pos.y;
                Some(Cow::Owned(crossterm::event::Event::Mouse(mm)))
            } else {
                None
            }
        }
        e => Some(Cow::Borrowed(e)),
    }
}

/// Render all windows.
///
pub fn render_windows<'a, T, S, M, RF, Err>(
    windows: &Windows<'_, S, M>,
    mut render_window: RF,
    area: Rect,
    buf: &mut Buffer,
    state: &mut WindowsState<T, S, M>,
) -> Result<(), Err>
where
    RF: FnMut(&mut T, Rect, &mut Buffer, &mut S) -> Result<(), Err>,
    T: ?Sized + 'a,
    S: WinBaseState + ?Sized + 'a,
    M: WindowManager,
{
    state.rc.manager.borrow_mut().set_offset(windows.offset);
    state.rc.manager.borrow_mut().set_area(area);

    let handles = state.rc.manager.borrow().handles();
    for handle in handles {
        state.run_for_window(handle, &mut |window, window_state| {
            windows.manager.render_init_window(
                handle,
                window_state.get_flags(),
                &mut state.rc.manager.borrow_mut(),
            );

            let (widget_area, mut tmp_buf) = windows
                .manager
                .render_init_buffer(handle, &mut state.rc.manager.borrow_mut());

            // window content
            render_window(window, widget_area, &mut tmp_buf, window_state)?;

            // window decorations
            windows.manager.render_window_frame(
                handle,
                &mut tmp_buf,
                &mut state.rc.manager.borrow_mut(),
            );

            // copy
            windows.manager.render_copy_buffer(
                &mut tmp_buf,
                area,
                buf,
                &mut state.rc.manager.borrow_mut(),
            );

            // keep allocation
            windows
                .manager
                .render_free_buffer(tmp_buf, &mut state.rc.manager.borrow_mut());

            Ok(())
        })?;
    }

    Ok(())
}
