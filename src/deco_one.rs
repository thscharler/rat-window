use crate::_private::NonExhaustive;
use crate::deco::deco_one_layout;
use crate::utils::fill_buf_area;
use crate::window_deco::{WindowDeco, WindowDecoStyle};
use crate::WindowState;
use rat_focus::HasFocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Widget};
use std::any::TypeId;

#[derive(Debug, Default, Clone)]
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

    #[allow(clippy::collapsible_else_if)]
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        deco_style: Option<&dyn WindowDecoStyle>,
        state: &mut dyn WindowState,
    ) {
        let one_style = OneStyle::default();
        let deco_style = deco_style
            .map(|v| v.downcast_ref::<OneStyle>())
            .unwrap_or(&one_style);

        deco_one_layout(area, deco_style.block.inner(area), state.window_mut());

        deco_style.block.clone().render(area, buf);

        let style = if state.window().focus.is_focused() {
            if let Some(focus_style) = deco_style.focus_style {
                focus_style
            } else if let Some(title_style) = deco_style.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        } else {
            if let Some(title_style) = deco_style.title_style {
                title_style
            } else {
                Style::new().black().on_white()
            }
        };
        let alignment = deco_style.title_alignment.unwrap_or_default();

        if let Some(cell) = buf.cell_mut((area.left(), area.top())) {
            cell.set_symbol("\u{2590}");
        }
        fill_buf_area(buf, state.window().area_title, " ", style);
        if let Some(cell) = buf.cell_mut((area.right() - 1, area.top())) {
            cell.set_symbol("\u{258C}");
        }

        let mut title_area = state.window().area_title;
        title_area.width = state.window().area_close.x.saturating_sub(title_area.x + 1);

        Text::from(state.window().title.as_str())
            .style(style)
            .alignment(alignment)
            .render(title_area, buf);

        Span::from("[\u{2A2F}]").render(state.window().area_close, buf);
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
