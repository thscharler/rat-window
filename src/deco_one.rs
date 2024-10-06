use crate::utils::fill_buf_area;
use crate::WindowState;
use rat_focus::HasFocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, StatefulWidget, Widget};

#[derive(Debug)]
pub(crate) struct OneDecoration<'a> {
    block: Block<'a>,
    title: Option<&'a str>,
    title_style: Option<Style>,
    title_alignment: Option<Alignment>,
    focus_style: Option<Style>,
}

impl<'a> OneDecoration<'a> {
    pub(crate) fn new() -> Self {
        Self {
            block: Block::bordered(),
            title: None,
            title_style: None,
            title_alignment: None,
            focus_style: None,
        }
    }

    pub(crate) fn block(mut self, block: Option<Block<'a>>) -> Self {
        if let Some(block) = block {
            self.block = block;
        }
        self
    }

    pub(crate) fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }

    pub(crate) fn title_style(mut self, style: Option<Style>) -> Self {
        self.title_style = style;
        self
    }

    pub(crate) fn title_alignment(mut self, align: Option<Alignment>) -> Self {
        self.title_alignment = align;
        self
    }

    pub(crate) fn focus_style(mut self, style: Option<Style>) -> Self {
        if let Some(focus_style) = style {
            self.focus_style = Some(focus_style);
        }
        self
    }
}

impl<'a> StatefulWidget for OneDecoration<'a> {
    type State = WindowState;

    #[allow(clippy::collapsible_else_if)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.inner = self.block.inner(area);

        if state.closeable {
            state.areas[WindowState::CLOSE] = Rect::new(area.right() - 5, area.top(), 3, 1);
        } else {
            state.areas[WindowState::CLOSE] = Rect::default();
        }
        if state.moveable {
            state.areas[WindowState::MOVE] =
                Rect::new(area.left() + 1, area.top(), area.width - 2, 1);
        } else {
            state.areas[WindowState::MOVE] = Rect::default();
        }
        if state.resizable {
            state.areas[WindowState::RESIZE_TOP] = Rect::new(area.left() + 1, area.top(), 0, 1);
            state.areas[WindowState::RESIZE_RIGHT] =
                Rect::new(area.right() - 1, area.top() + 1, 1, area.height - 2);
            state.areas[WindowState::RESIZE_BOTTOM] =
                Rect::new(area.left() + 1, area.bottom() - 1, area.width - 2, 1);
            state.areas[WindowState::RESIZE_LEFT] =
                Rect::new(area.left(), area.top() + 1, 1, area.height - 2);
            state.areas[WindowState::RESIZE_TOP_LEFT] = Rect::new(area.left(), area.top(), 1, 1);
            state.areas[WindowState::RESIZE_TOP_RIGHT] =
                Rect::new(area.right() - 1, area.top(), 1, 1);
            state.areas[WindowState::RESIZE_BOTTOM_RIGHT] =
                Rect::new(area.right() - 1, area.bottom() - 1, 1, 1);
            state.areas[WindowState::RESIZE_BOTTOM_LEFT] =
                Rect::new(area.left(), area.bottom() - 1, 1, 1);
        } else {
            state.areas[WindowState::RESIZE_TOP] = Rect::default();
            state.areas[WindowState::RESIZE_RIGHT] = Rect::default();
            state.areas[WindowState::RESIZE_BOTTOM] = Rect::default();
            state.areas[WindowState::RESIZE_LEFT] = Rect::default();
            state.areas[WindowState::RESIZE_TOP_LEFT] = Rect::default();
            state.areas[WindowState::RESIZE_TOP_RIGHT] = Rect::default();
            state.areas[WindowState::RESIZE_BOTTOM_RIGHT] = Rect::default();
            state.areas[WindowState::RESIZE_BOTTOM_LEFT] = Rect::default();
        }
        state.areas[WindowState::TITLE] = Rect::new(area.left() + 1, area.top(), area.width - 2, 1);

        self.block.render(area, buf);

        let style = if state.focus.is_focused() {
            if let Some(focus_style) = self.focus_style {
                focus_style
            } else if let Some(title_style) = self.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        } else {
            if let Some(title_style) = self.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        };
        let alignment = self.title_alignment.unwrap_or_default();
        let title = self.title.unwrap_or("%__%))--");

        if let Some(cell) = buf.cell_mut((area.left(), area.top())) {
            cell.set_symbol("\u{2590}");
        }
        fill_buf_area(buf, state.areas[WindowState::TITLE], " ", style);
        if let Some(cell) = buf.cell_mut((area.right() - 1, area.top())) {
            cell.set_symbol("\u{258C}");
        }

        Text::from(title)
            .style(style)
            .alignment(alignment)
            .render(state.areas[WindowState::TITLE], buf);

        Span::from("[\u{2A2F}]").render(state.areas[WindowState::CLOSE], buf);
    }
}
