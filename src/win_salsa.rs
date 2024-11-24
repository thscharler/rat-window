use crate::win_base::WinBaseState;
use crate::{relocate_event, WindowManager, WindowManagerState, Windows, WindowsState};
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_salsa::timer::TimeOut;
use rat_salsa::{AppContext, AppState, AppWidget, Control, RenderContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::Any;
use std::cmp::max;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

pub trait WinSalsaWidget<Global, Message, Error>:
    AppWidget<Global, Message, Error, State = dyn WinSalsaState<Global, Message, Error>>
where
    Global: 'static,
    Message: 'static + Send + Debug,
    Error: 'static + Send + Debug,
{
}

pub trait WinSalsaState<Global, Message, Error>: WinBaseState + Any + Debug
where
    Self: AppState<Global, Message, Error>,
    Global: 'static,
    Message: 'static + Send + Debug,
    Error: 'static + Send + Debug,
{
}

impl<'a, M: WindowManager, Global, Message, Error> AppWidget<Global, Message, Error>
    for Windows<'a, dyn WinSalsaState<Global, Message, Error>, M>
where
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
    type State = WindowsState<
        dyn AppWidget<Global, Message, Error, State = dyn WinSalsaState<Global, Message, Error>>,
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
        state.rc.manager.borrow_mut().set_offset(self.offset);
        state.rc.manager.borrow_mut().set_area(area);

        let handles = state.rc.manager.borrow().handles();
        for handle in handles {
            state.run_for_window(handle, &mut |window, window_state| {
                self.manager.render_init_window(
                    handle,
                    window_state.get_flags(),
                    &mut state.rc.manager.borrow_mut(),
                );

                let (widget_area, mut tmp_buf) = self
                    .manager
                    .render_init_buffer(handle, &mut state.rc.manager.borrow_mut());

                // window content
                window.render(widget_area, &mut tmp_buf, window_state, ctx)?;

                // window decorations
                self.manager.render_window_frame(
                    handle,
                    &mut tmp_buf,
                    &mut state.rc.manager.borrow_mut(),
                );

                // copy
                self.manager.render_copy_buffer(
                    &mut tmp_buf,
                    area,
                    buf,
                    &mut state.rc.manager.borrow_mut(),
                );

                // keep allocation
                self.manager
                    .render_free_buffer(tmp_buf, &mut state.rc.manager.borrow_mut());

                Ok(())
            })?;
        }

        Ok(())
    }
}

impl<Global, Message, Error, M> AppState<Global, Message, Error>
    for WindowsState<
        dyn AppWidget<Global, Message, Error, State = dyn WinSalsaState<Global, Message, Error>>,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >
where
    M: WindowManager,
    M::State: HandleEvent<crossterm::event::Event, Regular, Outcome>,
    Message: 'static + Send,
    Error: 'static + Send,
{
    fn init(&mut self, ctx: &mut AppContext<'_, Global, Message, Error>) -> Result<(), Error> {
        for handle in self.handles().into_iter().rev() {
            self.run_for_window(handle, &mut |_window, window_state| window_state.init(ctx))?;
        }
        Ok(())
    }

    fn timer(
        &mut self,
        event: &TimeOut,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        for handle in self.handles().into_iter().rev() {
            let r = self.run_for_window(handle, &mut |_window, window_state| {
                window_state.timer(event, ctx)
            });
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
        let Some(relocated) = relocate_event(self.rc.manager.borrow().deref(), event) else {
            return Ok(Control::Continue);
        };
        let relocated = relocated.as_ref();

        // forward to window-manager
        let r0 = self
            .rc
            .manager
            .borrow_mut()
            .deref_mut()
            .handle(relocated, Regular);

        // forward to all windows
        let r1 = if !r0.is_consumed() {
            'f: {
                for handle in self.handles().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |_window, window_state| {
                        window_state.crossterm(relocated, ctx)
                    })?;
                    if r.is_consumed() {
                        break 'f Ok(r);
                    }
                }
                Ok(Control::Continue)
            }?
        } else {
            Control::Continue
        };

        Ok(max(r1, r0.into()))
    }

    fn message(
        &mut self,
        event: &mut Message,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        for handle in self.handles().into_iter().rev() {
            let r = self.run_for_window(handle, &mut |_window, window_state| {
                window_state.message(event, ctx)
            });
            if r.is_consumed() {
                return r;
            }
        }
        Ok(Control::Continue)
    }
}
