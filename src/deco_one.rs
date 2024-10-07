use crate::_private::NonExhaustive;
use crate::deco::deco_one_layout;
use crate::utils::fill_buf_area;
use crate::window_deco::{WindowDeco, WindowDecoStyle};
use crate::{WindowState, WindowUserState};
use rat_focus::HasFocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget};
use std::any::TypeId;

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

impl WindowDeco for One {
    fn style_id(&self) -> TypeId {
        TypeId::of::<OneStyle>()
    }

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        win_style: Option<&dyn WindowDecoStyle>,
        win_state: &mut WindowState,
        _win_user: &mut dyn WindowUserState,
    ) {
        let one_style = OneStyle::default();
        let win_style = win_style
            .map(|v| v.downcast_ref::<OneStyle>())
            .unwrap_or(&one_style);

        deco_one_layout(area, win_style.block.inner(area), win_state);

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

impl One {
    pub fn new() -> Self {
        Self
    }
}

impl WindowDecoStyle for OneStyle {}

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
