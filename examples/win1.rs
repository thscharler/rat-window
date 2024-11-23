use crate::max_win::{MaxWin, MaxWinState};
use crate::min_win::{MinWin, MinWinState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{FocusBuilder, FocusContainer, HasFocus};
use rat_window::{DecoOne, DecoOneState, WinCtState, WinCtWidget, Windows, WindowsState};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::cmp::max;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        win: WindowsState::new(DecoOneState::new()),
    };
    state.win.focus().set(true);

    run_ui(
        "win1",
        handle_windows,
        repaint_windows,
        &mut data,
        &mut state,
    )
}

struct Data {}

struct State {
    win: WindowsState<dyn WinCtWidget, dyn WinCtState, DecoOne>,
}

fn repaint_windows(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let hlayout = Layout::horizontal([
        Constraint::Length(5),
        Constraint::Fill(1), //
        Constraint::Length(5),
    ])
    .split(layout[1]);

    frame.buffer_mut().set_style(hlayout[1], THEME.gray(0));

    Windows::new(
        DecoOne::new()
            .block(
                Block::bordered()
                    .border_type(BorderType::Thick)
                    // .borders(Borders::TOP)
                    .border_style(THEME.black(1)),
            )
            .title_style(THEME.black(1))
            .title_alignment(Alignment::Center)
            .focus_style(THEME.focus())
            .meta_style(THEME.secondary(2)),
    )
    .offset(Position::new(10, 10))
    .render(hlayout[1], frame.buffer_mut(), &mut state.win);

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
    // focus.enable_log();
    let f = focus.handle(event, Regular);

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let c = (rand::random::<u8>() % 26 + b'a') as char;
            let cstr = c.to_string();

            let mut minwin = MinWin;
            let mut minwin_state = MinWinState::new();
            minwin_state.set_fill(cstr);

            state.win.open_window(
                (minwin.into(), minwin_state.into()),
                Rect::new(10, 10, 15, 20),
            );
            Outcome::Changed
        }
        ct_event!(keycode press F(3)) => {
            state.win.open_window(
                (MaxWin.into(), MaxWinState::new().into()),
                Rect::new(10, 10, 20, 15),
            );
            Outcome::Changed
        }
        _ => Outcome::Continue,
    };

    let r = r.or_else(|| (&state.win).handle(event, Regular));

    Ok(max(f, r))
}

impl FocusContainer for State {
    fn build(&self, _builder: &mut FocusBuilder) {
        // builder.container(&self.win);
    }
}

// -------------------------------------------------------------

pub mod min_win {
    use crate::mini_salsa::theme::THEME;
    use crossterm::event::Event;
    use rat_event::{ct_event, HandleEvent, Outcome, Regular};
    use rat_window::{WinCtState, WinCtWidget, WinFlags, WinHandle, WinState, WinWidget};
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Position, Rect};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MinWin;

    impl WinCtWidget for MinWin {}

    impl From<MinWin> for Rc<RefCell<dyn WinCtWidget>> {
        fn from(value: MinWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    #[derive(Debug, Default)]
    pub struct MinWinState {
        fill: String,

        handle: Option<WinHandle>,
        win: WinFlags,
    }

    impl MinWinState {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set_fill(&mut self, fill: String) {
            self.fill = fill;
        }
    }

    impl From<MinWinState> for Rc<RefCell<dyn WinCtState>> {
        fn from(value: MinWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    impl WinWidget for MinWin {
        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinState) {
            let state = state.downcast_mut::<MinWinState>().expect("minwin-state");

            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                        cell.set_style(THEME.orange(1));
                        cell.set_symbol(state.fill.as_str());
                    }
                }
            }
        }
    }

    impl WinState for MinWinState {
        fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }

        fn as_dyn(&mut self) -> &mut dyn WinState {
            self
        }
    }

    impl WinCtState for MinWinState {}

    impl HandleEvent<Event, Regular, Outcome> for MinWinState {
        fn handle(&mut self, event: &Event, _qualifier: Regular) -> Outcome {
            match event {
                ct_event!(mouse down Left for _x,_y) => Outcome::Changed,
                _ => Outcome::Continue,
            }
        }
    }
}

pub mod max_win {
    use crate::mini_salsa::theme::THEME;
    use crossterm::event::Event;
    use rat_event::{ct_event, HandleEvent, Outcome, Regular};
    use rat_window::{WinCtState, WinCtWidget, WinFlags, WinHandle, WinState, WinWidget};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::text::Line;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MaxWin;

    impl WinCtWidget for MaxWin {}

    impl From<MaxWin> for Rc<RefCell<dyn WinCtWidget>> {
        fn from(value: MaxWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    #[derive(Debug, Default)]
    pub struct MaxWinState {
        msg: String,

        handle: Option<WinHandle>,
        win: WinFlags,
    }

    impl MaxWinState {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn message(mut self, message: String) -> Self {
            self.msg = message;
            self
        }
    }

    impl WinWidget for MaxWin {
        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut dyn WinState) {
            let state = state.downcast_mut::<MaxWinState>().expect("maxwin-state");

            buf.set_style(area, THEME.limegreen(1));
            Line::from(state.msg.as_ref()).render(area, buf);
        }
    }

    impl WinState for MaxWinState {
        fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }

        fn as_dyn(&mut self) -> &mut dyn WinState {
            self
        }
    }

    impl From<MaxWinState> for Rc<RefCell<dyn WinCtState>> {
        fn from(value: MaxWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    impl WinCtState for MaxWinState {}

    impl HandleEvent<Event, Regular, Outcome> for MaxWinState {
        fn handle(&mut self, event: &Event, _qualifier: Regular) -> Outcome {
            match event {
                ct_event!(mouse any for m) => {
                    self.msg = format!("{}:{}", m.column, m.row);
                    Outcome::Continue
                }
                _ => Outcome::Continue,
            }
        }
    }
}
