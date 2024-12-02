use crate::WinHandle;
use rat_event::{ConsumedEvent, Outcome};

/// Result of event handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowsOutcome {
    /// The given event has not been used at all.
    Continue,
    /// The event has been recognized, but the result was nil.
    /// Further processing for this event may stop.
    Unchanged,
    /// The event has been recognized and there is some change
    /// due to it.
    /// Further processing for this event may stop.
    /// Rendering the ui is advised.
    Changed,
    /// Currently moving.
    Moving(WinHandle),
    /// Currently resizing.
    Resizing(WinHandle),
    /// Snap to a region occurred.
    Snap(WinHandle, u16),
    /// Moved to front.
    ToFront(WinHandle),
    /// Moved
    Moved(WinHandle),
    /// Resized
    Resized(WinHandle),
}

impl ConsumedEvent for WindowsOutcome {
    fn is_consumed(&self) -> bool {
        *self != WindowsOutcome::Continue
    }
}

// Useful for converting most navigation/edit results.
impl From<bool> for WindowsOutcome {
    fn from(value: bool) -> Self {
        if value {
            WindowsOutcome::Changed
        } else {
            WindowsOutcome::Unchanged
        }
    }
}

impl From<WindowsOutcome> for Outcome {
    fn from(value: WindowsOutcome) -> Self {
        match value {
            WindowsOutcome::Continue => Outcome::Continue,
            WindowsOutcome::Unchanged => Outcome::Unchanged,
            WindowsOutcome::Changed => Outcome::Changed,
            WindowsOutcome::Snap(_, _) => Outcome::Changed,
            WindowsOutcome::ToFront(_) => Outcome::Changed,
            WindowsOutcome::Moving(_) => Outcome::Changed,
            WindowsOutcome::Moved(_) => Outcome::Changed,
            WindowsOutcome::Resizing(_) => Outcome::Changed,
            WindowsOutcome::Resized(_) => Outcome::Changed,
        }
    }
}

impl From<Outcome> for WindowsOutcome {
    fn from(value: Outcome) -> Self {
        match value {
            Outcome::Continue => WindowsOutcome::Continue,
            Outcome::Unchanged => WindowsOutcome::Unchanged,
            Outcome::Changed => WindowsOutcome::Changed,
        }
    }
}
