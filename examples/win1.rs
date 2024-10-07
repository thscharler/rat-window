use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use log::debug;
use rat_event::{ct_event, try_flow, HandleEvent, Outcome, Regular};
use rat_focus::HasFocusFlag;
use rat_window::deco::{One, OneStyle};
use rat_window::utils::fill_buf_area;
use rat_window::{Window, WindowBuilder, WindowState, WindowUserState, Windows, WindowsState};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, StatefulWidget, StatefulWidgetRef};
use ratatui::Frame;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        win: WindowsState::new().zero_offset(3, 3).deco(One),
    };

    run_ui(handle_windows, repaint_windows, &mut data, &mut state)
}

struct Data {}

type DynUserState = Box<dyn WindowUserState + 'static>;
type DynWindow = Box<dyn Window<DynUserState> + 'static>;

struct State {
    win: WindowsState<DynWindow, DynUserState>,
}

fn repaint_windows(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let layout = Layout::vertical([
        Constraint::Fill(1), //
        Constraint::Length(1),
    ])
    .split(area);

    let hlayout = Layout::horizontal([
        Constraint::Fill(1), //
        Constraint::Length(15),
    ])
    .split(layout[0]);

    // frame.buffer_mut().set_style(hlayout[0], THEME.black(2));
    fill_buf_area(frame.buffer_mut(), hlayout[0], " ", THEME.black(2));
    Windows::new()
        .deco(OneStyle {
            block: Block::bordered().border_type(BorderType::Rounded),
            title_style: Some(THEME.bluegreen(2)),
            title_alignment: Some(Alignment::Right),
            focus_style: Some(THEME.focus()),
            ..Default::default()
        })
        .render(hlayout[0], frame.buffer_mut(), &mut state.win);

    // state.win;

    Ok(())
}

fn handle_windows(
    event: &crossterm::event::Event,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    try_flow!(match event {
        ct_event!(keycode press F(2)) => {
            let c = (rand::random::<u8>() % 26 + b'a') as char;
            state.win.show(
                WindowBuilder::new(
                    MinWin::new().fill(c).boxed(),
                    MinWinState::new(format!(" {} ", rand::random::<u8>())).boxed(),
                )
                .area(Rect::new(10, 10, 15, 20)),
            );
            Outcome::Changed
        }
        _ => Outcome::Continue,
    });

    try_flow!(state.win.handle(event, Regular));

    Ok(Outcome::Continue)
}

#[derive(Debug, Default)]
struct MinWin {
    fill: char,
}

struct MinWinState {
    msg: String,
}

impl MinWin {
    fn new() -> Self {
        Self::default()
    }

    fn fill(mut self, fill: char) -> Self {
        self.fill = fill;
        self
    }

    fn boxed(self) -> DynWindow {
        Box::new(self)
    }
}

impl Window<DynUserState> for MinWin {
    fn state_id(&self) -> TypeId {
        TypeId::of::<MinWinState>()
    }

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        win_state: &mut WindowState,
        win_user: &mut DynUserState,
    ) {
        let win_user = win_user.downcast_mut::<MinWinState>();

        fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

        if win_state.is_focused() {
            (&win_user.msg).render(area, buf);
        } else {
            (&win_user.msg).render(area, buf);
        }
    }
}

impl WindowUserState for MinWinState {}

impl MinWinState {
    fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    fn boxed(self) -> DynUserState {
        Box::new(self)
    }
}
