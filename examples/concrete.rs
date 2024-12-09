use crate::min_win::{MinWin, MinWinState, MinWindows, MinWindowsState};
use crate::mini_salsa::text_input_mock::{TextInputMock, TextInputMockState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{Focus, FocusBuilder, FocusContainer};
use rat_widget::text::HasScreenCursor;
use rat_window::{DecoOne, DecoOneState, WinFlags};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::Frame;
use std::cell::RefCell;
use std::cmp::max;
use std::rc::Rc;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        focus: None,
        win: MinWindowsState::new(DecoOneState::new()),
        mock0: Default::default(),
        mock1: Default::default(),
    };

    run_ui(
        "concrete",
        handle_windows,
        repaint_windows,
        &mut data,
        &mut state,
    )
}

struct Data {}

struct State {
    focus: Option<Focus>,
    win: MinWindowsState,
    mock0: TextInputMockState,
    mock1: TextInputMockState,
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

    TextInputMock::default()
        .style(THEME.text_input())
        .focus_style(THEME.text_focus())
        .render(hlayout[0], frame.buffer_mut(), &mut state.mock0);
    TextInputMock::default()
        .style(THEME.text_input())
        .focus_style(THEME.text_focus())
        .render(hlayout[2], frame.buffer_mut(), &mut state.mock1);

    MinWindows::new(
        DecoOne::new()
            .block(
                Block::bordered()
                    .border_type(BorderType::Thick)
                    .border_style(THEME.black(1)),
            )
            .title_style(THEME.black(1))
            .title_alignment(Alignment::Center)
            .focus_style(THEME.focus())
            .config_style(THEME.secondary(2)),
    )
    .offset(Position::new(10, 10))
    .render(hlayout[1], frame.buffer_mut(), &mut state.win.clone());

    if let Some(cursor) = state.screen_cursor() {
        frame.set_cursor_position(cursor);
    }

    Ok(())
}

fn handle_windows(
    event: &crossterm::event::Event,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    // build focus
    let old_focus = state.focus.take();
    let mut focus = FocusBuilder::rebuild(state, old_focus);
    let f = focus.handle(event, Regular);
    state.focus = Some(focus);

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let minwin = MinWin;

            let minwin_state = MinWinState {
                f0: Default::default(),
                f1: Default::default(),
                f2: Default::default(),
                f3: Default::default(),
                handle: None,
                win: Default::default(),
            };

            let handle = state.win.open_window(
                Rc::new(RefCell::new(minwin)),
                Rc::new(RefCell::new(minwin_state)),
            );
            state.win.set_window_area(handle, Rect::new(10, 10, 15, 20));
            state.win.set_window_flags(
                handle,
                WinFlags {
                    title: format!("{:?}", handle),
                    ..Default::default()
                },
            );

            state
                .win
                .window_state(handle)
                .borrow_mut()
                .set_handle(handle);

            Outcome::Changed
        }
        _ => Outcome::Continue,
    };

    let r = r.or_else(|| state.win.handle(event, Regular).into());

    Ok(max(f, r))
}

impl HasScreenCursor for State {
    fn screen_cursor(&self) -> Option<(u16, u16)> {
        self.mock0
            .screen_cursor()
            .or(self.win.screen_cursor())
            .or(self.mock1.screen_cursor())
    }
}

impl FocusContainer for State {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.widget(&self.mock0);
        builder.container(&self.win);
        builder.widget(&self.mock1);
    }
}

// -------------------------------------------------------------

pub mod min_win {
    use crate::mini_salsa::text_input_mock::{TextInputMock, TextInputMockState};
    use crate::mini_salsa::theme::THEME;
    use rat_event::{ct_event, HandleEvent, Outcome, Regular};
    use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer};
    use rat_reloc::RelocatableState;
    use rat_widget::text::HasScreenCursor;
    use rat_window::event::WindowsOutcome;
    use rat_window::{
        fill_buffer, render_windows, DecoOne, DecoOneOutcome, DecoOneState, WinFlags, WinHandle,
        WindowManagerState, WindowMode, Windows, WindowsState,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Position, Rect};
    use ratatui::widgets::StatefulWidget;
    use std::fmt::Debug;
    use std::ops::{Deref, DerefMut};

    #[derive(Debug)]
    pub struct MinWin;

    #[derive(Debug, Default)]
    pub struct MinWinState {
        pub f0: TextInputMockState,
        pub f1: TextInputMockState,
        pub f2: TextInputMockState,
        pub f3: TextInputMockState,

        pub handle: Option<WinHandle>,
        pub win: WinFlags,
    }

    impl StatefulWidget for &MinWin {
        type State = MinWinState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            fill_buffer(" ", THEME.orange(0), area, buf);

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
        }
    }

    impl FocusContainer for MinWinState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.widget(&self.f0);
            builder.widget(&self.f1);
            builder.widget(&self.f2);
            builder.widget(&self.f3);
        }
    }

    impl RelocatableState for MinWinState {
        fn relocate(&mut self, shift: (i16, i16), clip: Rect) {
            self.f0.relocate(shift, clip);
            self.f1.relocate(shift, clip);
            self.f2.relocate(shift, clip);
            self.f3.relocate(shift, clip);
        }
    }

    impl HasScreenCursor for MinWinState {
        fn screen_cursor(&self) -> Option<(u16, u16)> {
            self.f0
                .screen_cursor()
                .or(self.f1.screen_cursor())
                .or(self.f2.screen_cursor())
                .or(self.f3.screen_cursor())
        }
    }

    impl MinWinState {
        pub fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        pub fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }
    }

    impl HandleEvent<crossterm::event::Event, Regular, Outcome> for MinWinState {
        fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
            match event {
                ct_event!(mouse down Left for _x,_y) => Outcome::Changed,
                _ => Outcome::Continue,
            }
        }
    }

    // There are only basic traits defined for Windows/WindowsState.
    //
    // In order to implement traits for some actual functionality we need
    // new-types for Windows and WindowsState. Plus we use Deref/DerefMut to
    // make those as close to invisible as possible.
    //
    // Then we can define
    // * StatefulWidget
    // * FocusContainer
    // * HandleEvent
    //
    // as we wish.

    #[repr(transparent)]
    #[derive(Debug)]
    pub struct MinWindows<'a>(Windows<'a, MinWinState, DecoOne>);

    #[repr(transparent)]
    #[derive(Debug)]
    pub struct MinWindowsState(WindowsState<MinWin, MinWinState, DecoOne>);

    impl<'a> Deref for MinWindows<'a> {
        type Target = Windows<'a, MinWinState, DecoOne>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a> DerefMut for MinWindows<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl Deref for MinWindowsState {
        type Target = WindowsState<MinWin, MinWinState, DecoOne>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for MinWindowsState {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl Clone for MinWindowsState {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<'a> MinWindows<'a> {
        pub fn new(manager: DecoOne) -> Self {
            Self(Windows::new(manager))
        }

        pub fn offset(mut self, offset: Position) -> Self {
            self.0.offset = offset;
            self
        }
    }

    impl MinWindowsState {
        pub fn new(window_manager_state: DecoOneState) -> Self {
            Self(WindowsState::new(window_manager_state))
        }
    }

    impl<'a> StatefulWidget for MinWindows<'a> {
        type State = MinWindowsState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            _ = render_windows(
                &self,
                |window, widget_area, buf, window_state| {
                    window.render(widget_area, buf, window_state);
                    Ok::<(), ()>(())
                },
                area,
                buf,
                state,
            );
        }
    }

    impl HasScreenCursor for MinWindowsState {
        fn screen_cursor(&self) -> Option<(u16, u16)> {
            // only have the windows themselves.
            let manager = self.rc.manager.borrow();
            if manager.mode() == WindowMode::Config {
                None
            } else {
                if let Some(handle) = manager.front_window() {
                    let window_state = self.window_state(handle);
                    let window_state = window_state.borrow();
                    window_state.screen_cursor()
                } else {
                    None
                }
            }
        }
    }

    impl FocusContainer for MinWindowsState {
        fn build(&self, builder: &mut FocusBuilder) {
            // only have the windows themselves.
            let manager = self.rc.manager.borrow();

            if manager.mode() == WindowMode::Config {
                for handle in self.handles_create() {
                    let frame = manager.window_frame(handle);
                    let has_focus = frame.as_has_focus();

                    // need the container for rendering the focus.
                    let container_end = builder.start(
                        Some(manager.window_container(handle)),
                        has_focus.area(),
                        has_focus.area_z(),
                    );
                    builder.widget(has_focus);
                    builder.end(container_end);
                }
            } else {
                if let Some(handle) = manager.front_window() {
                    let window_state = self.window_state(handle);
                    let win_area = self.window_area(handle);

                    // need the container for rendering the focus.
                    let container_end =
                        builder.start(Some(manager.window_container(handle)), win_area);

                    builder.container(window_state.borrow().deref());

                    builder.end(container_end);
                }
            }
        }

        fn container(&self) -> Option<ContainerFlag> {
            // container for all windows
            Some(self.rc.manager.borrow().container())
        }

        fn area(&self) -> Rect {
            Rect::default()
        }
    }

    impl HandleEvent<crossterm::event::Event, Regular, WindowsOutcome> for MinWindowsState {
        fn handle(
            &mut self,
            event: &crossterm::event::Event,
            _qualifier: Regular,
        ) -> WindowsOutcome {
            // forward to window-manager
            let r = self.rc.manager.borrow_mut().handle(event, Regular);
            match r {
                DecoOneOutcome::ToFront(h, old) => {
                    // transfer focus
                    if let Some(oh) = old {
                        let old_focus = self.window_focus(oh);
                        let focus = self.window_focus(h);

                        focus.set(old_focus.get());
                        old_focus.set(false);

                        let old_focus = self.window_container(oh);
                        let focus = self.window_container(h);
                        focus.set(old_focus.get());
                        old_focus.set(false);
                    }
                    WindowsOutcome::ToFront(h, old)
                }
                r => r.into(),
            }
        }
    }
}
