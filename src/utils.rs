use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Style;

/// Fill the given area of the buffer.
pub fn fill_buf_area(buf: &mut Buffer, area: Rect, symbol: &str, style: impl Into<Style>) {
    let style = style.into();

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.reset();
                cell.set_symbol(symbol);
                cell.set_style(style);
            }
        }
    }
}

/// Copy a tmp buffer to another buf.
/// The tmp-buffer is left/top shifted by h_shift/v_shift.
/// Everything is clipped to the target area.
pub(crate) fn copy_buffer(
    mut tmp: Buffer,
    h_shift: u16,
    v_shift: u16,
    area: Rect,
    buf: &mut Buffer,
) {
    // copy buffer
    for (cell_offset, cell) in tmp.content.drain(..).enumerate() {
        let tmp_row = tmp.area.x + cell_offset as u16 / tmp.area.width;
        let tmp_col = tmp.area.y + cell_offset as u16 % tmp.area.width;

        // clip
        if tmp_row < v_shift {
            continue;
        }
        if tmp_col < h_shift {
            continue;
        }
        if tmp_row - v_shift > area.height {
            continue;
        }
        if tmp_col - h_shift > area.width {
            continue;
        }

        let row = tmp_row - v_shift + area.y;
        let col = tmp_col - h_shift + area.y;

        if let Some(buf_cell) = buf.cell_mut((col, row)) {
            *buf_cell = cell;
        }
    }
}
