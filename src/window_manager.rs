use crate::{WinFlags, WinHandle, Windows, WindowsState};
use rat_focus::{ContainerFlag, FocusContainer, FocusFlag, HasFocus};
use rat_reloc::RelocatableState;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use std::ops::DerefMut;

pub trait WindowManager {
    /// State struct
    type State: WindowManagerState;

    /// Outcome type from event handling.
    type Outcome;

    /// Run calculations based on the currently set windows area and offset.
    fn render_init(&self, state: &mut Self::State);

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

    /// Maximum z-index used for windows.
    fn max_z(&self) -> u16;

    /// Set current window mode.
    fn set_mode(&mut self, mode: WindowMode);

    /// Current window mode.
    fn mode(&self) -> WindowMode;

    /// Container flag for all windows.
    fn container(&self) -> ContainerFlag;

    /// Container flag for the given window.
    fn window_container(&self, handle: WinHandle) -> ContainerFlag;

    /// Sometimes the window itself wants to act as a widget.
    fn window_focus(&self, handle: WinHandle) -> FocusFlag;

    /// Get the window frame widget.
    fn window_frame(&self, handle: WinHandle) -> &dyn WindowFrame;

    /// Insert a window into the window manager.
    fn insert_window(&mut self, handle: WinHandle);

    /// Remove a window from the window manager.
    fn remove_window(&mut self, handle: WinHandle);

    /// The active window area including the frame.
    fn window_area(&self, handle: WinHandle) -> Rect;

    /// The active window area including the frame.
    fn set_window_area(&mut self, handle: WinHandle, area: Rect);

    /// Window flags.
    fn window_flags(&self, handle: WinHandle) -> WinFlags;

    /// Window flags.
    fn set_window_flags(&mut self, handle: WinHandle, flags: WinFlags);

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
    fn window_snap_idx(&self, handle: WinHandle) -> Option<u16>;

    /// The snap-index of the window.
    ///
    /// __Panic__
    /// Panics when the index is out of bounds.
    fn set_window_snap_idx(&mut self, handle: WinHandle, idx: Option<u16>);

    /// Area for the window's content.
    fn window_widget_area(&self, handle: WinHandle) -> Rect;

    /// Return a list of the window handles
    /// in rendering order.
    fn handles_render(&self) -> Vec<WinHandle>;

    /// Move the focused window to front.
    fn focus_to_front(&mut self) -> bool;

    /// Focused window
    fn focused_window(&self) -> Option<WinHandle>;

    /// Move a window to front.
    fn window_to_front(&mut self, handle: WinHandle) -> bool;

    /// Get the front window handle
    fn front_window(&self) -> Option<WinHandle>;

    /// Window at the given __screen__ position.
    fn window_at(&self, pos: Position) -> Option<WinHandle>;

    /// Return the shift required to call [rat-reloc::RelocatableState]
    /// for a window.
    fn shift(&self) -> (i16, i16);
}

/// Mode for the window manager.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    /// Normal mode.
    #[default]
    Regular,
    /// Window configuration mode.
    ///
    /// When this mode is set size and position of the windows
    /// can be changed with keyboard actions.
    Config,
}

/// Trait for the window frame widget.
pub trait WindowFrame {
    /// Cast the window frame as focusable widget.
    fn as_has_focus(&self) -> &dyn HasFocus;

    /// Cast the window frame as a widget container.
    fn as_focus_container(&self) -> &dyn FocusContainer;
}

/// Helper function used to implement window rendering for a
/// specific window-type.
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
    S: RelocatableState + ?Sized + 'a,
    M: WindowManager,
{
    state.rc.manager.borrow_mut().set_offset(windows.offset);
    state.rc.manager.borrow_mut().set_area(area);

    windows
        .manager
        .render_init(state.rc.manager.borrow_mut().deref_mut());

    let handles = state.rc.manager.borrow().handles_render();
    for handle in handles {
        state.run_for_window(handle, &mut |window, window_state| {
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

            // relocate the window state to the target
            window_state.relocate(
                state.rc.manager.borrow().shift(),
                state.rc.manager.borrow().area(),
            );

            Ok(())
        })?;
    }

    Ok(())
}
