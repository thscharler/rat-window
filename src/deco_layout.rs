//!
//! Layout functions for window deco.
//!
use crate::WindowSysState;
use ratatui::layout::Rect;

/// Calculate the layout for Deco-One windows.
pub fn deco_one_layout(area: Rect, inner: Rect, win_state: &mut WindowSysState) {
    win_state.area = Rect::new(area.x, area.y, area.width, area.height);
    win_state.inner = Rect::new(inner.x, inner.y, inner.width, inner.height);

    if win_state.closeable {
        win_state.area_close = Rect::new(area.right() - 5, area.top(), 3, 1);
    } else {
        win_state.area_close = Rect::default();
    }
    if win_state.moveable {
        win_state.area_move = Rect::new(area.left() + 1, area.top(), area.width - 2, 1);
    } else {
        win_state.area_move = Rect::default();
    }
    if win_state.resizable {
        win_state.area_resize_top = Rect::new(area.left() + 1, area.top(), 0, 1);
        win_state.area_resize_right =
            Rect::new(area.right() - 1, area.top() + 1, 1, area.height - 2);
        win_state.area_resize_bottom =
            Rect::new(area.left() + 1, area.bottom() - 1, area.width - 2, 1);
        win_state.area_resize_left = Rect::new(area.left(), area.top() + 1, 1, area.height - 2);
        win_state.area_resize_top_left = Rect::new(area.left(), area.top(), 1, 1);
        win_state.area_resize_top_right = Rect::new(area.right() - 1, area.top(), 1, 1);
        win_state.area_resize_bottom_left = Rect::new(area.left(), area.bottom() - 1, 1, 1);
        win_state.area_resize_bottom_right = Rect::new(area.right() - 1, area.bottom() - 1, 1, 1);
    } else {
        win_state.area_resize_top = Rect::default();
        win_state.area_resize_right = Rect::default();
        win_state.area_resize_bottom = Rect::default();
        win_state.area_resize_left = Rect::default();
        win_state.area_resize_top_left = Rect::default();
        win_state.area_resize_top_right = Rect::default();
        win_state.area_resize_bottom_left = Rect::default();
        win_state.area_resize_bottom_right = Rect::default();
    }

    win_state.area_title = Rect::new(area.left() + 1, area.top(), area.width - 2, 1);
}
