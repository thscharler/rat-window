use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use log::debug;
use rat_event::{ct_event, try_flow, HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{FocusBuilder, HasFocus, HasFocusFlag};
use rat_window::deco::{One, OneStyle};
use rat_window::utils::fill_buf_area;
use rat_window::{
    DynEventUserState, DynEventWindow, DynUserState, DynWindow, EventUserState, Window,
    WindowBuilder, WindowState, WindowUserState, Windows, WindowsState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::any::TypeId;
use std::fmt::Debug;

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

struct State {
    win: WindowsState<DynEventWindow, DynEventUserState>,
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

    fn boxed(self) -> DynEventWindow {
        Box::new(self)
    }
}

impl Window<DynEventUserState> for MinWin {
    fn state_id(&self) -> TypeId {
        TypeId::of::<MinWinState>()
    }

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        win_state: &mut WindowState,
        win_user: &mut DynEventUserState,
    ) {
        let win_user = win_user.downcast_mut::<MinWinState>();

        fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

        if win_state.focus.is_focused() {
            (&win_user.msg).render(area, buf);
        } else {
            (&win_user.msg).render(area, buf);
        }
    }
}

impl WindowUserState for MinWinState {}

impl EventUserState for MinWinState {}

impl HasFocus for MinWinState {
    fn build(&self, builder: &mut FocusBuilder) {
        todo!()
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for MinWinState {
    fn handle(&mut self, event: &crossterm::event::Event, qualifier: MouseOnly) -> Outcome {
        debug!("win1 mouse only");
        Outcome::Continue
    }
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for MinWinState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        debug!("win1 regular");
        self.msg = format!("{:?}", event);
        Outcome::Changed
    }
}

impl MinWinState {
    fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    fn boxed(self) -> DynEventUserState {
        Box::new(self)
    }
}
