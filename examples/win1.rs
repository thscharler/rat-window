use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, MouseOnly, Outcome, Regular};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, HasFocusFlag};
use rat_window::deco::{One, OneStyle};
use rat_window::utils::fill_buf_area;
use rat_window::{
    DynEventUserState, DynEventWindow, EventUserState, Window, WindowBuilder, WindowState,
    WindowUserState, Windows, WindowsState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::any::TypeId;
use std::cmp::max;
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
    let mut b = FocusBuilder::new(None).enable_log();
    b.container(state);
    let mut focus = b.build();
    focus.enable_log();
    let f = focus.handle(event, Regular);

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let c = (rand::random::<u8>() % 26 + b'a') as char;
            state.win.show(
                WindowBuilder::new(
                    MinWin::new().fill(c).boxed(),
                    MinWinState::new(c, format!(" {} ", rand::random::<u8>())).boxed(),
                )
                .area(Rect::new(10, 10, 15, 20)),
            );
            Outcome::Changed
        }
        _ => Outcome::Continue,
    };

    let r = r.or_else(|| state.win.handle(event, Regular));

    Ok(max(f, r))
}

impl HasFocus for State {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.container(&self.win);
    }
}

// -------------------------------------------------------------

#[derive(Debug, Default)]
struct MinWin {
    fill: char,
}

struct MinWinState {
    msg: String,

    window: WindowState,
    focus: FocusFlag,
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

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut DynEventUserState) {
        let state = state.downcast_mut::<MinWinState>();

        fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

        if state.window_state().focus.is_focused() {
            (&state.msg).render(area, buf);
        } else {
            (&state.msg).render(area, buf);
        }
    }
}

impl WindowUserState for MinWinState {
    fn window_state(&self) -> &WindowState {
        &self.window
    }

    fn window_state_mut(&mut self) -> &mut WindowState {
        &mut self.window
    }
}

impl EventUserState for MinWinState {}

impl HasFocus for MinWinState {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.widget(&self.focus);
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for MinWinState {
    fn handle(&mut self, _event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        // debug!("win1 mouse only");
        Outcome::Continue
    }
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for MinWinState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        // debug!("win1 regular");
        self.msg = format!("{:?}", event);
        Outcome::Changed
    }
}

impl MinWinState {
    fn new(c: char, msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            window: Default::default(),
            focus: FocusFlag::named(format!("{}", c).as_str()),
        }
    }

    fn boxed(self) -> DynEventUserState {
        Box::new(self)
    }
}
