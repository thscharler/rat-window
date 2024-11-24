use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::prelude::Style;
use ratatui::style::Stylize;
use std::mem;

pub fn fill_buffer(symbol: &str, style: Style, area: Rect, buf: &mut Buffer) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(symbol);
                cell.set_style(style);
            }
        }
    }
}

/// Copy a tmp buffer to another buf.
/// The tmp-buffer is left/top shifted by h_shift/v_shift.
/// Everything is clipped to the target area.
pub(crate) fn copy_buffer(tmp: &mut Buffer, shift: Position, area: Rect, buf: &mut Buffer) {
    // copy buffer
    for (cell_offset, cell) in tmp.content.drain(..).enumerate() {
        let r_y = cell_offset as u16 / tmp.area.width;
        let r_x = cell_offset as u16 % tmp.area.width;

        let tmp_y = tmp.area.y + r_y;
        let tmp_x = tmp.area.x + r_x;

        // clip
        if tmp_y < shift.y {
            continue;
        }
        if tmp_x < shift.x {
            continue;
        }
        if tmp_y - shift.y >= area.height {
            continue;
        }
        if tmp_x - shift.x >= area.width {
            continue;
        }

        let y = tmp_y - shift.y + area.y;
        let x = tmp_x - shift.x + area.x;

        if let Some(buf_cell) = buf.cell_mut((x, y)) {
            if cell != Cell::EMPTY {
                *buf_cell = cell;
            }
        }
    }
}

/// Returns a new style with fg and bg swapped.
///
/// This is not the same as setting Style::reversed().
/// The latter sends special controls to the terminal,
/// the former just swaps.
pub(crate) fn revert_style(mut style: Style) -> Style {
    if style.fg.is_some() || style.bg.is_some() {
        mem::swap(&mut style.fg, &mut style.bg);
        style
    } else {
        style.black().on_white()
    }
}
