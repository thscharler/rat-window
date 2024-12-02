use crate::min_win::{MinWin, MinWinState, MinWindows, MinWindowsState};
use crate::mini_salsa::text_input_mock::{TextInputMock, TextInputMockState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{Focus, FocusBuilder, FocusContainer};
use rat_window::{DecoOne, DecoOneState, WinFlags, WindowManagerState};
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
            .meta_style(THEME.secondary(2)),
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
    // build focus
    let old_focus = state.focus.take();
    let mut focus = FocusBuilder::rebuild(state, old_focus);
    let f = focus.handle(event, Regular);
    state.focus = Some(focus);

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let minwin = MinWin;

            let fd = state.focus.as_mut().expect("some").clone_destruct();
            let minwin_state = MinWinState {
                focus_flags: fd.0,
                areas: fd.1,
                z_rects: fd.2,
                navigations: fd.3,
                containers: fd.4,
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

impl FocusContainer for State {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.widget(&self.mock0);
        builder.container(&self.win);
        builder.widget(&self.mock1);
    }
}

// -------------------------------------------------------------

pub mod min_win {
    use crate::mini_salsa::theme::THEME;
    use rat_event::{ct_event, HandleEvent, Outcome, Regular};
    use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer, FocusFlag, Navigation, ZRect};
    use rat_window::event::WindowsOutcome;
    use rat_window::{
        fill_buffer, relocate_event, render_windows, DecoOne, DecoOneOutcome, DecoOneState,
        WinFlags, WinHandle, WindowManagerState, Windows, WindowsState,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Position, Rect};
    use ratatui::text::Span;
    use ratatui::widgets::{StatefulWidget, Widget};
    use std::fmt::Debug;
    use std::ops::{Deref, DerefMut, Range};

    #[derive(Debug)]
    pub struct MinWin;

    #[derive(Debug, Default)]
    pub struct MinWinState {
        pub focus_flags: Vec<FocusFlag>,
        pub areas: Vec<Rect>,
        pub z_rects: Vec<Vec<ZRect>>,
        pub navigations: Vec<Navigation>,
        pub containers: Vec<(ContainerFlag, Rect, Range<usize>)>,

        pub handle: Option<WinHandle>,
        pub win: WinFlags,
    }

    impl StatefulWidget for &MinWin {
        type State = MinWinState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            fill_buffer(" ", THEME.orange(0), area, buf);

            let mut info_area = Rect::new(area.x, area.y, area.width, 1);
            for (idx, focus) in state.focus_flags.iter().enumerate() {
                Span::from(format!("{}:{} {}", idx, focus.name(), focus.get()))
                    .render(info_area, buf);
                info_area.y += 1;

                for zrect in state.z_rects[idx].iter() {
                    Span::from(format!(
                        "    {}:{}z{}+{}+{} ",
                        zrect.x, zrect.y, zrect.z, zrect.width, zrect.height
                    ))
                    .render(info_area, buf);
                    info_area.y += 1;
                }
            }
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

    impl FocusContainer for MinWindowsState {
        fn build(&self, builder: &mut FocusBuilder) {
            // only have the windows themselves.
            let manager = self.rc.manager.borrow();

            if let Some(handle) = manager.front_window() {
                let frame = manager.window_frame(handle);
                let has_focus = frame.as_has_focus();

                // need the container for rendering the focus.
                let container_end =
                    builder.start(Some(manager.window_container(handle)), has_focus.area());
                builder.widget(has_focus);
                builder.end(container_end);
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
            let Some(event) = relocate_event(self.rc.manager.borrow().deref(), event) else {
                return WindowsOutcome::Continue;
            };

            // forward to window-manager
            let r = self.rc.manager.borrow_mut().handle(event.as_ref(), Regular);
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
