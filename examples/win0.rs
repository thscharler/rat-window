use crate::max_win::{MaxWin, MaxWinState};
use crate::min_win::{MinWin, MinWinState};
use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::{Focus, FocusBuilder, FocusContainer};
use rat_window::{DecoOne, DecoOneState, WinFlags, WinState, WinWidget, Windows, WindowsState};
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
        "win0",
        handle_windows,
        repaint_windows,
        &mut data,
        &mut state,
    )
}

struct Data {}

struct State {
    focus: Option<Focus>,
    win: WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, DecoOne>,
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

    Windows::<dyn WinState>::new(
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

    // let fd = focus.clone_destruct();
    // debug!("{:#?}", fd);
    // for handle in state.win.handles_render() {
    //     let win_state = state.win.window_state(handle);
    //     let mut win_state = win_state.borrow_mut();
    //     if let Some(minwin) = win_state.deref_mut().downcast_mut::<MinWinState>() {
    //         minwin.focus_flags = fd.0.clone();
    //         minwin.areas = fd.1.clone();
    //         minwin.z_rects = fd.2.clone();
    //         minwin.navigations = fd.3.clone();
    //         minwin.containers = fd.4.clone();
    //     }
    // }

    let r = match event {
        ct_event!(keycode press F(2)) => {
            let minwin = MinWin;

            let fd = focus.clone_destruct();
            let minwin_state = MinWinState {
                focus_flags: fd.0,
                areas: fd.1,
                z_rects: fd.2,
                navigations: fd.3,
                containers: fd.4,
                handle: None,
                win: Default::default(),
            };

            let handle = state.win.open_window(minwin.into(), minwin_state.into());
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
                .for_mut::<MinWinState>(|w| w.set_handle(handle));

            Outcome::Changed
        }
        ct_event!(keycode press F(3)) => {
            let maxwin = MaxWin;
            let maxwin_state = MaxWinState::new(state.win.clone());

            let handle = state.win.open_window(maxwin.into(), maxwin_state.into());
            state.win.set_window_area(handle, Rect::new(10, 10, 20, 15));
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
                .for_mut(|s: &mut MaxWinState| s.set_handle(handle));

            Outcome::Changed
        }
        _ => Outcome::Continue,
    };

    let r = r.or_else(|| state.win.handle(event, Regular));

    Ok(max(f, r))
}

impl FocusContainer for State {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.container(&self.win);
    }
}

// -------------------------------------------------------------

pub mod min_win {
    use crate::mini_salsa::theme::THEME;
    use crossterm::event::Event;
    use rat_event::{ct_event, HandleEvent, Outcome, Regular};
    use rat_focus::{ContainerFlag, FocusFlag, Navigation, ZRect};
    use rat_window::{fill_buffer, WinFlags, WinHandle, WinState, WinWidget};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::text::Span;
    use ratatui::widgets::Widget;
    use std::cell::RefCell;
    use std::ops::Range;
    use std::rc::Rc;

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

    impl From<MinWin> for Rc<RefCell<dyn WinWidget<State = dyn WinState>>> {
        fn from(value: MinWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    impl WinWidget for MinWin {
        type State = dyn WinState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MinWinState>().expect("minwin-state");

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

    impl WinState for MinWinState {}

    impl From<MinWinState> for Rc<RefCell<dyn WinState>> {
        fn from(value: MinWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

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
    use rat_window::{
        fill_buffer, DecoOne, WinFlags, WinHandle, WinState, WinWidget, WindowsState,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::text::Line;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MaxWin;

    impl WinWidget for MaxWin {
        type State = dyn WinState;

        fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let state = state.downcast_mut::<MaxWinState>().expect("maxwin-state");

            fill_buffer(" ", THEME.deepblue(0), area, buf);

            let mut info_area = Rect::new(area.x, area.y, area.width, 1);
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

    impl From<MaxWin> for Rc<RefCell<dyn WinWidget<State = dyn WinState>>> {
        fn from(value: MaxWin) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

    #[derive(Debug)]
    pub struct MaxWinState {
        msg: String,

        windows: WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, DecoOne>,

        handle: Option<WinHandle>,
        win: WinFlags,
    }

    impl MaxWinState {
        pub fn new(
            windows: WindowsState<dyn WinWidget<State = dyn WinState>, dyn WinState, DecoOne>,
        ) -> Self {
            Self {
                msg: "".to_string(),
                windows,
                handle: None,
                win: Default::default(),
            }
        }

        pub fn message(mut self, message: String) -> Self {
            self.msg = message;
            self
        }

        pub fn set_handle(&mut self, handle: WinHandle) {
            self.handle = Some(handle);
        }

        pub fn get_flags(&self) -> WinFlags {
            self.win.clone()
        }
    }

    impl WinState for MaxWinState {}

    impl From<MaxWinState> for Rc<RefCell<dyn WinState>> {
        fn from(value: MaxWinState) -> Self {
            Rc::new(RefCell::new(value))
        }
    }

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
