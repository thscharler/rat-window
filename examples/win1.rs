use crate::max_win::{MaxWin, MaxWinState};
use crate::min_win::{MinWin, MinWinState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{FocusBuilder, HasFocus};
use rat_window::box_dyn_event::{DynEventUserState, DynEventWindow};
use rat_window::deco::{One, OneStyle};
use rat_window::utils::fill_buf_area;
use rat_window::{WindowBuilder, Windows, WindowsState};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::cmp::max;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        win: WindowsState::new().zero_offset(3, 3).deco(One),
    };

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
            block: Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(THEME.white[1]).bg(THEME.black[2])),
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
        ct_event!(keycode press F(3)) => {
            let c = (rand::random::<u8>() % 26 + b'a') as char;
            state.win.show(
                WindowBuilder::new(
                    MaxWin::new().boxed(),
                    MaxWinState::new(c, format!(" {} ", rand::random::<u8>())).boxed(),
                )
                .area(Rect::new(10, 10, 20, 15)),
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

pub mod min_win {
    use rat_event::{HandleEvent, MouseOnly, Outcome, Regular};
    use rat_focus::{FocusBuilder, FocusFlag, HasFocus, HasFocusFlag};
    use rat_window::box_dyn_event::{DynEventUserState, DynEventWindow, EventUserState};
    use rat_window::utils::fill_buf_area;
    use rat_window::{Window, WindowState, WindowSysState};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::{Style, Widget};
    use ratatui::widgets::StatefulWidgetRef;
    use std::any::TypeId;

    #[derive(Debug, Default)]
    pub struct MinWin {
        fill: char,
    }

    #[derive(Debug)]
    pub struct MinWinState {
        msg: String,

        window: WindowSysState,
        focus: FocusFlag,
    }

    impl MinWin {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn fill(mut self, fill: char) -> Self {
            self.fill = fill;
            self
        }

        pub fn boxed(self) -> DynEventWindow {
            Box::new(self)
        }
    }

    impl Window for MinWin {
        fn state_id(&self) -> TypeId {
            TypeId::of::<MinWinState>()
        }
    }

    impl StatefulWidgetRef for MinWin {
        type State = DynEventUserState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MinWinState>();

            fill_buf_area(buf, area, &self.fill.to_string(), Style::default());

            if state.window().focus.is_focused() {
                (&state.msg).render(area, buf);
            } else {
                (&state.msg).render(area, buf);
            }
        }
    }

    impl EventUserState for MinWinState {}

    impl WindowState for MinWinState {
        fn window(&self) -> &WindowSysState {
            &self.window
        }

        fn window_mut(&mut self) -> &mut WindowSysState {
            &mut self.window
        }
    }

    impl HasFocus for MinWinState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.widget(&self.focus);
        }
    }

    impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for MinWinState {
        fn handle(&mut self, _event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
            Outcome::Continue
        }
    }

    impl HandleEvent<crossterm::event::Event, Regular, Outcome> for MinWinState {
        fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
            self.msg = format!("{:?}", event);
            Outcome::Changed
        }
    }

    impl MinWinState {
        pub fn new(c: char, msg: impl Into<String>) -> Self {
            Self {
                msg: msg.into(),
                window: Default::default(),
                focus: FocusFlag::named(format!("{}", c).as_str()),
            }
        }

        pub fn boxed(self) -> DynEventUserState {
            Box::new(self)
        }
    }
}

pub mod max_win {
    use rat_event::{ct_event, HandleEvent, MouseOnly, Outcome, Regular};
    use rat_focus::{FocusBuilder, FocusFlag, HasFocus, HasFocusFlag};
    use rat_window::box_dyn_event::{DynEventUserState, DynEventWindow, EventUserState};
    use rat_window::utils::fill_buf_area;
    use rat_window::{Window, WindowState, WindowSysState};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::style::Style;
    use ratatui::widgets::StatefulWidgetRef;
    use std::any::TypeId;

    #[derive(Debug, Default)]
    pub struct MaxWin {}

    #[derive(Debug)]
    pub struct MaxWinState {
        msg: String,

        window: WindowSysState,
        focus: FocusFlag,
    }

    impl MaxWin {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn boxed(self) -> DynEventWindow {
            Box::new(self)
        }
    }

    impl Window for MaxWin {
        fn state_id(&self) -> TypeId {
            TypeId::of::<MaxWinState>()
        }
    }

    impl StatefulWidgetRef for MaxWin {
        type State = DynEventUserState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MaxWinState>();

            fill_buf_area(buf, area, " ", Style::default());

            if state.window().focus.is_focused() {
                (&state.msg).render(area, buf);
            } else {
                (&state.msg).render(area, buf);
            }
        }
    }

    impl EventUserState for MaxWinState {}

    impl WindowState for MaxWinState {
        fn window(&self) -> &WindowSysState {
            &self.window
        }

        fn window_mut(&mut self) -> &mut WindowSysState {
            &mut self.window
        }
    }

    impl HasFocus for MaxWinState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.widget(&self.focus);
        }
    }

    impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for MaxWinState {
        fn handle(&mut self, _event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
            Outcome::Continue
        }
    }

    impl HandleEvent<crossterm::event::Event, Regular, Outcome> for MaxWinState {
        fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
            match event {
                ct_event!(key press c) => {
                    self.msg = format!("Key {}", c);
                    Outcome::Changed
                }
                ct_event!(mouse down Left for _x,_y) => {
                    self.msg = "clicky".into();
                    Outcome::Changed
                }
                ct_event!(mouse down Right for _x,_y) => {
                    self.msg = "clacky".into();
                    Outcome::Changed
                }
                _ => Outcome::Continue,
            }
        }
    }

    impl MaxWinState {
        pub fn new(c: char, msg: impl Into<String>) -> Self {
            Self {
                msg: msg.into(),
                window: WindowSysState {
                    title: "MAX".to_string(),
                    ..Default::default()
                },
                focus: FocusFlag::named(format!("{}", c).as_str()),
            }
        }

        pub fn boxed(self) -> DynEventUserState {
            Box::new(self)
        }
    }
}
