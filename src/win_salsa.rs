use crate::win_base::WinBaseState;
use crate::{relocate_event, WinFlags, WindowManager, WindowManagerState, Windows, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_salsa::timer::TimeOut;
use rat_salsa::{AppContext, AppState, AppWidget, Control, RenderContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::Any;
use std::ops::Deref;

pub trait WinSalsaState<Global, Message, Error>:
    WinBaseState + AppState<Global, Message, Error> + Any
where
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
    /// Get a copy of the windows flags governing general
    /// behaviour of the window.
    fn get_flags(&self) -> WinFlags;

    /// Return self as dyn WinState.
    fn as_dyn(&mut self) -> &mut dyn WinSalsaState<Global, Message, Error>;
}

impl<'a, Global, Message, Error, M> AppWidget<Global, Message, Error>
    for Windows<
        'a,
        dyn AppWidget<Global, Message, Error, State = dyn WinSalsaState<Global, Message, Error>>,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >
where
    M: WindowManager,
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
    type State = WindowsState<
        dyn WinSalsaWidget<
            Global,
            Message,
            Error,
            State = dyn WinSalsaState<Global, Message, Error>,
        >,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >;

    fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
        ctx: &mut RenderContext<'_, Global>,
    ) -> Result<(), Error> {
        let mut manager_state = state.manager_state.borrow_mut();

        manager_state.set_offset(self.offset);
        manager_state.set_area(area);

        for handle in manager_state.windows().iter().copied() {
            let (window, window_state) = state.window(handle);

            let mut window = window.borrow();
            let mut window_state = window_state.borrow_mut();

            self.manager
                .render_init_window(handle, window_state.get_flags(), &mut manager_state);

            let (widget_area, mut tmp_buf) =
                self.manager.render_init_buffer(handle, &mut manager_state);

            // window content
            window.render(widget_area, &mut tmp_buf, window_state.as_dyn(), ctx)?;

            // window decorations
            self.manager
                .render_window_frame(handle, &mut tmp_buf, &mut manager_state);

            // copy
            self.manager
                .render_copy_buffer(&mut tmp_buf, area, buf, &mut manager_state);

            // keep allocation
            self.manager.render_free_buffer(tmp_buf, &mut manager_state);
        }

        Ok(())
    }
}

impl<Global, Message, Error, T, M> AppState<Global, Message, Error>
    for WindowsState<T, dyn WinSalsaState<Global, Message, Error>, M>
where
    Message: 'static + Send,
    Error: 'static + Send,
    T: WinSalsaWidget<Global, Message, Error> + ?Sized + 'static,
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
{
    fn init(&mut self, ctx: &mut AppContext<'_, Global, Message, Error>) -> Result<(), Error> {
        for handle in self.windows().into_iter().rev() {
            self.run_for_window(handle, &mut |window| window.init(ctx))?;
        }
        Ok(())
    }

    fn timer(
        &mut self,
        event: &TimeOut,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        for handle in self.windows().into_iter().rev() {
            let r = self.run_for_window(handle, &mut |window| window.timer(event, ctx));
            if r.is_consumed() {
                return r;
            }
        }
        Ok(Control::Continue)
    }

    fn crossterm(
        &mut self,
        event: &crossterm::event::Event,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        let Some(relocated) = relocate_event(self.manager_state.borrow().deref(), event) else {
            return Ok(Control::Continue);
        };

        // forward to window-manager
        let r: Control<Message> = self
            .manager_state
            .borrow_mut()
            .handle(relocated.as_ref(), Regular)
            .into();

        let r = r.or_else_try(|| {
            // forward to all windows
            'f: {
                for handle in self.windows().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |window| {
                        window.crossterm(relocated.as_ref(), ctx)
                    })?;
                    if r.is_consumed() {
                        break 'f Ok(r);
                    }
                }
                Ok(Control::Continue)
            }
        })?;

        Ok(r)
    }

    fn message(
        &mut self,
        event: &mut Message,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        for handle in self.windows().into_iter().rev() {
            let r = self.run_for_window(handle, &mut |window| window.message(event, ctx));
            if r.is_consumed() {
                return r;
            }
        }
        Ok(Control::Continue)
    }
}
