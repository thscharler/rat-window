use crate::_private::NonExhaustive;
use crate::utils::fill_buf_area;
use crate::window_style::{WindowFrame, WindowFrameStyle};
use crate::WindowState;
use rat_focus::HasFocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, StatefulWidgetRef, Widget};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct One;

#[derive(Debug, Clone)]
pub struct OneStyle {
    pub block: Block<'static>,
    pub title_style: Option<Style>,
    pub title_alignment: Option<Alignment>,
    pub focus_style: Option<Style>,

    pub non_exhaustive: NonExhaustive,
}

impl WindowFrame for One {}

impl One {
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidgetRef for One {
    type State = (Rc<RefCell<WindowState>>, Rc<dyn WindowFrameStyle>);

    #[allow(clippy::collapsible_else_if)]
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let win_style = state.1.as_ref().downcast_ref::<OneStyle>();
        let mut win_state = state.0.borrow_mut();

        {
            win_state.area = Rect::new(area.x, area.y, area.width, area.height);
            let inner = win_style.block.inner(area);
            win_state.inner = Rect::new(inner.x, inner.y, inner.width, inner.height);
        }

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
            win_state.area_resize_bottom_right =
                Rect::new(area.right() - 1, area.bottom() - 1, 1, 1);
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

        win_style.block.clone().render(area, buf);

        let style = if win_state.focus.is_focused() {
            if let Some(focus_style) = win_style.focus_style {
                focus_style
            } else if let Some(title_style) = win_style.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        } else {
            if let Some(title_style) = win_style.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        };
        let alignment = win_style.title_alignment.unwrap_or_default();

        if let Some(cell) = buf.cell_mut((area.left(), area.top())) {
            cell.set_symbol("\u{2590}");
        }
        fill_buf_area(buf, win_state.area_title, " ", style);
        if let Some(cell) = buf.cell_mut((area.right() - 1, area.top())) {
            cell.set_symbol("\u{258C}");
        }

        Text::from(win_state.title.as_str())
            .style(style)
            .alignment(alignment)
            .render(win_state.area_title, buf);

        Span::from("[\u{2A2F}]").render(win_state.area_close, buf);
    }
}

impl WindowFrameStyle for OneStyle {}

impl Default for OneStyle {
    fn default() -> Self {
        Self {
            block: Default::default(),
            title_style: None,
            title_alignment: None,
            focus_style: None,
            non_exhaustive: NonExhaustive,
        }
    }
}

impl OneStyle {
    pub fn block(mut self, block: Option<Block<'static>>) -> Self {
        if let Some(block) = block {
            self.block = block;
        }
        self
    }

    pub fn title_style(mut self, style: Option<Style>) -> Self {
        self.title_style = style;
        self
    }

    pub fn title_alignment(mut self, align: Option<Alignment>) -> Self {
        self.title_alignment = align;
        self
    }

    pub fn focus_style(mut self, style: Option<Style>) -> Self {
        if let Some(focus_style) = style {
            self.focus_style = Some(focus_style);
        }
        self
    }
}
