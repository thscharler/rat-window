use crate::window_manager::{relocate_event, WindowManager};
use crate::{WinState, WinWidget, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use std::ops::Deref;

/// Trait for a window with event handling.
pub trait WinCtWidget: WinWidget {}

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

impl<T, M> HandleEvent<crossterm::event::Event, Regular, Outcome>
    for &WindowsState<T, dyn WinCtState, M>
where
    T: WinWidget + ?Sized + 'static,
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        let Some(relocated) = relocate_event(self.manager_state.borrow().deref(), event) else {
            return Outcome::Continue;
        };

        // forward to window-manager
        let r = self
            .manager_state
            .borrow_mut()
            .handle(relocated.as_ref(), Regular);

        let r = r.or_else(|| {
            // forward to all windows
            'f: {
                for handle in self.windows().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |window| {
                        window.handle(relocated.as_ref(), Regular)
                    });
                    if r.is_consumed() {
                        break 'f r;
                    }
                }
                Outcome::Continue
            }
        });

        r
    }
}
