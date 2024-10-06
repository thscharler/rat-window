use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use crossterm::event::{MouseEvent, MouseEventKind};
use log::debug;
use rat_event::{ct_event, try_flow, HandleEvent, Outcome, Regular};
use rat_focus::{FocusFlag, HasFocusFlag};
use rat_window::deco::{One, OneStyle};
use rat_window::utils::fill_buf_area;
use rat_window::{Window, WindowState, Windows, WindowsState};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, StatefulWidget, StatefulWidgetRef};
use ratatui::Frame;
use std::fmt::Debug;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        win: WindowsState::new()
            // .zero(10, 10)
            .deco(One)
            .deco_style(OneStyle {
                block: Block::bordered().border_type(BorderType::Rounded),
                title_style: Some(THEME.bluegreen(2)),
                title_alignment: Some(Alignment::Right),
                focus_style: Some(THEME.focus()),
                ..Default::default()
            }),
    };

    run_ui(handle_windows, repaint_windows, &mut data, &mut state)
}

struct Data {}

struct State {
    win: WindowsState<Box<dyn Window + 'static>>,
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
    Windows::new().render(hlayout[0], frame.buffer_mut(), &mut state.win);

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
            // let c = (rand::random::<u8>().saturating_sub(161).saturating_add(32)) as char;
            state.win.show_at(
                MinWin {
                    fill: 'o',
                    focus: Default::default(),
                    area: Default::default(),
                }
                .boxed(),
                WindowState::default().title("one".into()),
                Rect::new(20, 20, 20, 20),
            );
            Outcome::Changed
        }
        _ => Outcome::Continue,
    });

    match event {
        crossterm::event::Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            ..
        }) => {}
        crossterm::event::Event::Mouse(m) => {
            debug!("*NO FUN {:?} {:?}", m.column, m.row);
        }
        _ => {}
    }

    try_flow!(state.win.handle(event, Regular));

    Ok(Outcome::Continue)
}

#[derive(Debug, Default)]
struct MinWin {
    pub fill: char,

    pub focus: FocusFlag,
    pub area: Rect,
}

impl MinWin {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

impl HasFocusFlag for MinWin {
    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl Window for MinWin {}

impl StatefulWidgetRef for MinWin {
    type State = WindowState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

        if self.is_focused() {
            "MINWIN".render(area, buf);
        } else {
            "minwin".render(area, buf);
        }
    }
}
