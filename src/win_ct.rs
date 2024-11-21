use crate::{WinState, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};

///
/// Trait for a window with event handling.
///
/// Reuses [WinState] and adds event handling.
///
pub trait WinCtState
where
    Self: WinState,
    Self: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
}

impl HandleEvent<crossterm::event::Event, Regular, Outcome> for &WindowsState<dyn WinCtState> {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        use crossterm::event::Event;

        // forward to window-manager
        let r = match event {
            Event::Mouse(m) => {
                // can only convert a subset of the mouse-events.
                if let Some(m_relocated) = self.relocate_mouse_event(m) {
                    self.manager_state
                        .borrow_mut()
                        .handle(&Event::Mouse(m_relocated), Regular)
                } else {
                    Outcome::Continue
                }
            }
            event => self.manager_state.borrow_mut().handle(event, Regular),
        };

        let r = r.or_else(|| {
            match event {
                Event::Mouse(m) => {
                    // can only convert a subset of the mouse-events.
                    if let Some(m_relocated) = self.relocate_mouse_event(m) {
                        let event_relocated = Event::Mouse(m_relocated);

                        // forward to all windows
                        'f: {
                            for handle in self.windows().into_iter().rev() {
                                let r = self.run_for_window(handle, &mut |window| {
                                    window.handle(&event_relocated, Regular)
                                });
                                if r.is_consumed() {
                                    break 'f r;
                                }
                            }
                            Outcome::Continue
                        }
                    } else {
                        Outcome::Continue
                    }
                }
                event => {
                    // forward to all windows
                    'f: {
                        for handle in self.windows().into_iter().rev() {
                            let r = self
                                .run_for_window(handle, &mut |window| window.handle(event, Regular));
                            if r.is_consumed() {
                                break 'f r;
                            }
                        }
                        Outcome::Continue
                    }
                }
            }
        });

        r
    }
}
