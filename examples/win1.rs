use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use log::debug;
use rat_event::{ct_event, try_flow, Outcome};
use rat_focus::{FocusFlag, HasFocusFlag};
use rat_window::{Window, Windows, WindowsState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::Frame;
use std::fmt::Debug;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        win: WindowsState::default(),
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
    istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    Windows::new()
        .block(Block::bordered().border_style(Style::default().fg(istate.theme.deepblue[2])))
        .focus_style(Style::new().black().on_red())
        .render(area, frame.buffer_mut(), &mut state.win);

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
            state.win.show_at(
                Box::new(MinWin {
                    focus: Default::default(),
                    area: Default::default(),
                }),
                Rect::new(2, 2, 20, 20),
            );
            Outcome::Changed
        }
        _ => Outcome::Continue,
    });

    Ok(Outcome::Continue)
}

#[derive(Debug)]
struct MinWin {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocusFlag for MinWin {
    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl Window for MinWin {
    fn title(&self) -> Option<&str> {
        debug!("title called");
        Some("m i n   w i n")
    }

    fn is_closeable(&self) -> bool {
        true
    }

    fn is_resizable(&self) -> bool {
        true
    }

    fn is_moveable(&self) -> bool {
        true
    }

    fn is_modal(&self) -> bool {
        false
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.area = area;

        if self.is_focused() {
            "MINWIN".render(area, buf);
        } else {
            "minwin".render(area, buf);
        }
    }
}
