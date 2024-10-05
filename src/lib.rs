mod utils;

use crate::one::OneDecoration;
use crate::utils::copy_buffer;
use log::debug;
use rat_focus::{FocusFlag, HasFocusFlag};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::StatefulWidget;
use ratatui::style::Style;
use ratatui::widgets::Block;
use std::any::Any;
use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// Trait for a window.
pub trait Window: Any + HasFocusFlag + Debug {
    /// Window title.
    fn title(&self) -> Option<&str> {
        Some("")
    }

    /// Can close
    fn is_closeable(&self) -> bool {
        true
    }

    /// Can resize
    fn is_resizable(&self) -> bool {
        true
    }

    /// Can move
    fn is_moveable(&self) -> bool {
        true
    }

    /// Modal?
    fn is_modal(&self) -> bool {
        false
    }

    /// Draws the current state of the widget in the given buffer. That is the only method required
    /// to implement a custom widget.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}

/// Window handler
#[derive(Debug)]
pub struct Windows<'a, T>
where
    T: Window,
{
    block: Option<Block<'a>>,
    title_style: Option<Style>,
    title_alignment: Option<Alignment>,
    focus_style: Option<Style>,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
pub struct WindowsState<T>
where
    T: Window,
{
    // last rendered area for windowing.
    // read-only
    pub area: Rect,

    // offset of the window pane.
    // there are no negative coordinates.
    // this offset makes good for it.
    pub zero: (u16, u16),

    // window widget
    win: Vec<T>,
    // areas. x,y have zero added.
    win_area: Vec<Rect>,
    // other window state
    win_state: Vec<RefCell<WindowState>>,
}

const CLOSE: usize = 0;
const MOVE: usize = 1;
const RESIZE_TOP: usize = 2;
const RESIZE_RIGHT: usize = 3;
const RESIZE_BOTTOM: usize = 4;
const RESIZE_LEFT: usize = 5;
const RESIZE_TOP_LEFT: usize = 6;
const RESIZE_TOP_RIGHT: usize = 7;
const RESIZE_BOTTOM_RIGHT: usize = 8;
const RESIZE_BOTTOM_LEFT: usize = 9;
const TITLE: usize = 10;

#[derive(Debug, Default)]
pub struct WindowState {
    pub area: Rect,
    pub inner: Rect,
    pub areas: [Rect; 11],
    pub focus: FocusFlag,
    pub modal: bool,
    pub closeable: bool,
    pub resizable: bool,
    pub moveable: bool,
}

impl Window for Box<dyn Window + 'static> {
    fn title(&self) -> Option<&str> {
        self.as_ref().title()
    }

    fn is_closeable(&self) -> bool {
        self.as_ref().is_closeable()
    }

    fn is_resizable(&self) -> bool {
        self.as_ref().is_resizable()
    }

    fn is_moveable(&self) -> bool {
        self.as_ref().is_moveable()
    }

    fn is_modal(&self) -> bool {
        self.as_ref().is_modal()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.as_mut().render(area, buf);
    }
}

impl HasFocusFlag for Box<dyn Window + 'static> {
    fn focus(&self) -> FocusFlag {
        self.as_ref().focus()
    }

    fn area(&self) -> Rect {
        self.as_ref().area()
    }
}

impl<'a, T> Windows<'a, T>
where
    T: Window,
{
    pub fn new() -> Self {
        Self {
            block: None,
            title_style: None,
            title_alignment: None,
            focus_style: None,
            _phantom: Default::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn focus_style(mut self, focused: Style) -> Self {
        self.focus_style = Some(focused);
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = Some(style);
        self
    }

    pub fn title_alignment(mut self, align: Alignment) -> Self {
        self.title_alignment = Some(align);
        self
    }
}

impl<'a, T> StatefulWidget for Windows<'a, T>
where
    T: Window,
{
    type State = WindowsState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;

        // necessary buffer
        let mut tmp_area = Rect::new(state.zero.0, state.zero.1, 0, 0);
        for area in state.win_area.iter() {
            tmp_area = tmp_area.union(*area);
        }

        let mut tmp = Buffer::empty(tmp_area);

        for ((win, state), area) in state
            .win
            .iter_mut()
            .zip(state.win_state.iter())
            .zip(state.win_area.iter())
        {
            let mut state = state.borrow_mut();

            state.closeable = win.is_closeable();
            state.moveable = win.is_moveable();
            state.resizable = win.is_resizable();
            state.modal = win.is_modal();

            // decorations
            OneDecoration::new()
                .block(self.block.clone())
                .title(win.title())
                .title_style(self.title_style)
                .title_alignment(self.title_alignment)
                .focus_style(self.focus_style)
                .render(*area, &mut tmp, state.deref_mut());

            // content
            win.render(state.inner, &mut tmp);
        }

        copy_buffer(tmp, state.zero.0, state.zero.1, area, buf);
    }
}

impl<T> Default for WindowsState<T>
where
    T: Window,
{
    fn default() -> Self {
        Self {
            area: Default::default(),
            zero: (0, 0),
            win: vec![],
            win_area: vec![],
            win_state: vec![],
        }
    }
}

impl<T> WindowsState<T>
where
    T: Window,
{
    /// Show within bounds.
    pub fn show_at(&mut self, w: T, bounds: Rect) {
        self.win_state.push(RefCell::new(WindowState {
            area: Default::default(),
            inner: Default::default(),
            areas: [Rect::default(); 11],
            focus: w.focus(),
            modal: w.is_modal(),
            closeable: w.is_closeable(),
            resizable: w.is_resizable(),
            moveable: w.is_moveable(),
        }));
        self.win_area.push(self.shift_in(bounds));
        self.win.push(w);
    }

    pub fn show(&mut self, w: T) {
        self.show_at(w, self.area);
    }

    fn shift_in(&self, mut rect: Rect) -> Rect {
        rect.x += self.zero.0;
        rect.y += self.zero.1;
        rect
    }
}

mod one {
    use crate::utils::fill_buf_area;
    use crate::{
        WindowState, CLOSE, MOVE, RESIZE_BOTTOM, RESIZE_BOTTOM_LEFT, RESIZE_BOTTOM_RIGHT,
        RESIZE_LEFT, RESIZE_RIGHT, RESIZE_TOP, RESIZE_TOP_LEFT, RESIZE_TOP_RIGHT, TITLE,
    };
    use log::debug;
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

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            state.area = area;
            state.inner = self.block.inner(area);

            if state.closeable {
                state.areas[CLOSE] = Rect::new(area.right() - 5, area.top(), 3, 1);
            } else {
                state.areas[CLOSE] = Rect::default();
            }
            if state.moveable {
                state.areas[MOVE] = Rect::new(area.left() + 1, area.top(), area.width - 2, 1);
            } else {
                state.areas[MOVE] = Rect::default();
            }
            if state.resizable {
                state.areas[RESIZE_TOP] = Rect::new(area.left() + 1, area.top(), 0, 1);
                state.areas[RESIZE_RIGHT] =
                    Rect::new(area.right() - 1, area.top() + 1, 1, area.height - 2);
                state.areas[RESIZE_BOTTOM] =
                    Rect::new(area.left() + 1, area.bottom() - 1, area.width - 2, 1);
                state.areas[RESIZE_LEFT] =
                    Rect::new(area.left(), area.top() + 1, 1, area.height - 2);
                state.areas[RESIZE_TOP_LEFT] = Rect::new(area.left(), area.top(), 1, 1);
                state.areas[RESIZE_TOP_RIGHT] = Rect::new(area.right() - 1, area.top(), 1, 1);
                state.areas[RESIZE_BOTTOM_RIGHT] =
                    Rect::new(area.right() - 1, area.bottom() - 1, 1, 1);
                state.areas[RESIZE_BOTTOM_LEFT] = Rect::new(area.left(), area.bottom() - 1, 1, 1);
            } else {
                state.areas[RESIZE_TOP] = Rect::default();
                state.areas[RESIZE_RIGHT] = Rect::default();
                state.areas[RESIZE_BOTTOM] = Rect::default();
                state.areas[RESIZE_LEFT] = Rect::default();
                state.areas[RESIZE_TOP_LEFT] = Rect::default();
                state.areas[RESIZE_TOP_RIGHT] = Rect::default();
                state.areas[RESIZE_BOTTOM_RIGHT] = Rect::default();
                state.areas[RESIZE_BOTTOM_LEFT] = Rect::default();
            }
            state.areas[TITLE] = Rect::new(area.left() + 1, area.top(), area.width - 2, 1);

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

            let title = if let Some(title) = self.title {
                title
            } else {
                "%__%))--"
            };

            if let Some(cell) = buf.cell_mut((area.left(), area.top())) {
                cell.set_symbol("\u{2590}");
            }
            fill_buf_area(buf, state.areas[TITLE], " ", style);
            if let Some(cell) = buf.cell_mut((area.right() - 1, area.top())) {
                cell.set_symbol("\u{258C}");
            }

            debug!("title {:?}", title);

            Text::from(title)
                .style(style)
                .alignment(alignment)
                .render(state.areas[TITLE], buf);

            Span::from("[\u{2A2F}]").render(state.areas[CLOSE], buf);
        }
    }
}
