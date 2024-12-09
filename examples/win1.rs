use crate::max_win::{MaxWin, MaxWinState};
use crate::min_win::{MinWin, MinWinState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use log::debug;
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{Focus, FocusBuilder, FocusContainer};
use rat_window::{
    DecoOne, DecoOneState, WinCtState, WinCtWidget, WindowMode, Windows, WindowsState,
};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::cmp::max;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        focus: None,
        win: WindowsState::new(DecoOneState::new()),
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
    focus: Option<Focus>,
    win: WindowsState<dyn WinCtWidget<State = dyn WinCtState>, dyn WinCtState, DecoOne>,
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

    Windows::<dyn WinCtState>::new(
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
            .config_style(THEME.secondary(2)),
    )
    .offset(Position::new(10, 10))
    .render(hlayout[1], frame.buffer_mut(), &mut state.win.clone());

    Ok(())
}

fn handle_windows(
    event: &crossterm::event::Event,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let old_focus = state.focus.take();
    let mut focus = FocusBuilder::rebuild(state, old_focus);
    focus.enable_log();

    debug!("focus {:#?}", focus);

    let f = focus.handle(event, Regular);

    state.focus = Some(focus);

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let c = (rand::random::<u8>() % 26 + b'a') as char;
            let cstr = c.to_string();

            let minwin = MinWin;
            let mut minwin_state = MinWinState::new();
            minwin_state.set_fill(cstr);

            let handle = state.win.open_window(minwin.into(), minwin_state.into());
            state.win.set_window_area(handle, Rect::new(10, 10, 15, 20));

            Outcome::Changed
        }
        ct_event!(keycode press F(3)) => {
            let maxwin = MaxWin;
            let maxwin_state = MaxWinState::new(state.win.clone());

            let handle = state.win.open_window(maxwin.into(), maxwin_state.into());
            state.win.set_window_area(handle, Rect::new(10, 10, 20, 15));

            Outcome::Changed
        }
        ct_event!(keycode press F(8)) => match state.win.mode() {
            WindowMode::Regular => {
                state.win.set_mode(WindowMode::Config);
                Outcome::Changed
            }
            WindowMode::Config => {
                state.win.set_mode(WindowMode::Regular);
                Outcome::Changed
            }
        },
        _ => Outcome::Continue,
    };

    let r = r.or_else(|| state.win.handle(event, Regular).into());

    Ok(max(f, r))
}

impl FocusContainer for State {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.container(&self.win);
    }
}

// -------------------------------------------------------------

pub mod min_win {
    use crate::mini_salsa::text_input_mock::{TextInputMock, TextInputMockState};
    use crate::mini_salsa::theme::THEME;
    use crossterm::event::Event;
    use rat_cursor::HasScreenCursor;
    use rat_event::{HandleEvent, Outcome, Regular};
    use rat_focus::{FocusBuilder, FocusContainer};
    use rat_reloc::RelocatableState;
    use rat_window::{fill_buffer, WinCtState, WinCtWidget, WinFlags, WinHandle};
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Position, Rect};
    use ratatui::widgets::StatefulWidget;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MinWin;

    impl WinCtWidget for MinWin {
        type State = dyn WinCtState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MinWinState>().expect("minwin-state");

            fill_buffer(" ", THEME.orange(0), area, buf);
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut(Position::new(x, area.y)) {
                    cell.set_style(THEME.orange(1));
                    cell.set_symbol(state.fill.as_str());
                }
            }

            let mock_area = Rect::new(area.x + 1, area.y + 1, area.width * 2 / 3, 1);
            TextInputMock::default()
                .style(THEME.text_input())
                .focus_style(THEME.text_focus())
                .render(mock_area, buf, &mut state.m0);
        }
    }

    impl From<MinWin> for Rc<RefCell<dyn WinCtWidget<State = dyn WinCtState>>> {
        fn from(value: MinWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    #[derive(Debug, Default)]
    pub struct MinWinState {
        fill: String,

        m0: TextInputMockState,

        handle: Option<WinHandle>,
        win: WinFlags,
    }

    impl RelocatableState for MinWinState {
        fn relocate(&mut self, shift: (i16, i16), clip: Rect) {
            self.m0.relocate(shift, clip);
        }
    }

    impl HasScreenCursor for MinWinState {
        fn screen_cursor(&self) -> Option<(u16, u16)> {
            self.m0.screen_cursor()
        }
    }

    impl FocusContainer for MinWinState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.widget(&self.m0);
        }
    }

    impl WinCtState for MinWinState {
        fn as_focus_container(&self) -> &dyn FocusContainer {
            self
        }
    }

    impl MinWinState {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set_fill(&mut self, fill: String) {
            self.fill = fill;
        }

        pub fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        pub fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }
    }

    impl From<MinWinState> for Rc<RefCell<dyn WinCtState>> {
        fn from(value: MinWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    impl HandleEvent<Event, Regular, Outcome> for MinWinState {
        fn handle(&mut self, _event: &Event, _qualifier: Regular) -> Outcome {
            // ???
            Outcome::Continue
        }
    }
}

pub mod max_win {
    use crate::mini_salsa::text_input_mock::{TextInputMock, TextInputMockState};
    use crate::mini_salsa::theme::THEME;
    use crossterm::event::Event;
    use rat_cursor::HasScreenCursor;
    use rat_event::{HandleEvent, Outcome, Regular};
    use rat_focus::{FocusBuilder, FocusContainer};
    use rat_reloc::RelocatableState;
    use rat_window::{
        fill_buffer, DecoOne, WinCtState, WinCtWidget, WinFlags, WinHandle, WindowsState,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::text::Line;
    use ratatui::widgets::StatefulWidget;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MaxWin;

    impl WinCtWidget for MaxWin {
        type State = dyn WinCtState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MaxWinState>().expect("maxwin-state");

            fill_buffer(" ", THEME.deepblue(0), area, buf);

            let mut t_area = Rect::new(area.x, area.y + 1, area.width * 2 / 3, 1);
            TextInputMock::default()
                .style(THEME.text_input())
                .focus_style(THEME.text_focus())
                .render(t_area, buf, &mut state.f0);
            t_area.y += 2;

            TextInputMock::default()
                .style(THEME.text_input())
                .focus_style(THEME.text_focus())
                .render(t_area, buf, &mut state.f1);
            t_area.y += 2;

            TextInputMock::default()
                .style(THEME.text_input())
                .focus_style(THEME.text_focus())
                .render(t_area, buf, &mut state.f2);
            t_area.y += 2;

            TextInputMock::default()
                .style(THEME.text_input())
                .focus_style(THEME.text_focus())
                .render(t_area, buf, &mut state.f3);
            t_area.y += 2;

            let mut info_area = Rect::new(area.x, t_area.y, area.width, 1);
            for handle in state.windows.handles_render() {
                let win_area = state.windows.window_area(handle);

                Line::from(format!(
                    "{:?}: {}:{}+{}+{}",
                    handle, win_area.x, win_area.y, win_area.width, win_area.height
                ))
                .render(info_area, buf);

                info_area.y += 1;
            }
        }
    }

    impl From<MaxWin> for Rc<RefCell<dyn WinCtWidget<State = dyn WinCtState>>> {
        fn from(value: MaxWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    #[derive(Debug)]
    pub struct MaxWinState {
        pub f0: TextInputMockState,
        pub f1: TextInputMockState,
        pub f2: TextInputMockState,
        pub f3: TextInputMockState,

        windows: WindowsState<dyn WinCtWidget<State = dyn WinCtState>, dyn WinCtState, DecoOne>,
        handle: Option<WinHandle>,
        win: WinFlags,
    }

    impl RelocatableState for MaxWinState {
        fn relocate(&mut self, shift: (i16, i16), clip: Rect) {
            self.f0.relocate(shift, clip);
            self.f1.relocate(shift, clip);
            self.f2.relocate(shift, clip);
            self.f3.relocate(shift, clip);
        }
    }

    impl HasScreenCursor for MaxWinState {
        fn screen_cursor(&self) -> Option<(u16, u16)> {
            self.f0
                .screen_cursor()
                .or(self.f1.screen_cursor())
                .or(self.f2.screen_cursor())
                .or(self.f3.screen_cursor())
        }
    }

    impl FocusContainer for MaxWinState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.widget(&self.f0);
            builder.widget(&self.f1);
            builder.widget(&self.f2);
            builder.widget(&self.f3);
        }
    }

    impl WinCtState for MaxWinState {
        fn as_focus_container(&self) -> &dyn FocusContainer {
            self
        }
    }

    impl MaxWinState {
        pub fn new(
            windows: WindowsState<dyn WinCtWidget<State = dyn WinCtState>, dyn WinCtState, DecoOne>,
        ) -> Self {
            Self {
                f0: Default::default(),
                f1: Default::default(),
                f2: Default::default(),
                f3: Default::default(),
                windows,
                handle: None,
                win: Default::default(),
            }
        }

        pub fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        pub fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }
    }

    impl From<MaxWinState> for Rc<RefCell<dyn WinCtState>> {
        fn from(value: MaxWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    impl HandleEvent<Event, Regular, Outcome> for MaxWinState {
        fn handle(&mut self, _event: &Event, _qualifier: Regular) -> Outcome {
            // TODO??
            Outcome::Continue
        }
    }
}
