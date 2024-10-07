use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
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
use std::any::TypeId;
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

struct State {
    win: WindowsState<Box<dyn Window + 'static>, Box<dyn WindowUserState + 'static>>,
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

    fn boxed(self) -> Box<dyn Window> {
        Box::new(self)
    }
}

impl Window for MinWin {
    fn state_id(&self) -> TypeId {
        TypeId::of::<MinWinState>()
    }
}

impl StatefulWidgetRef for MinWin {
    type State = (Rc<RefCell<WindowState>>, Rc<RefCell<dyn WindowUserState>>);

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        (window_state, user_state): &mut Self::State,
    ) {
        let window_state = window_state.borrow();

        let mut user_state = user_state.borrow_mut();
        let user_state = user_state.downcast_box_dyn_mut::<MinWinState>();

        fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

        if window_state.is_focused() {
            (&user_state.msg).render(area, buf);
        } else {
            (&user_state.msg).render(area, buf);
        }
    }
}

impl WindowUserState for MinWinState {}

impl MinWinState {
    fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    fn boxed(self) -> Box<dyn WindowUserState> {
        Box::new(self)
    }
}
