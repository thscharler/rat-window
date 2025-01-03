use crate::event::WindowsOutcome;
use crate::{render_windows, WindowManager, WindowManagerState, WindowMode, Windows, WindowsState};
use rat_cursor::HasScreenCursor;
use rat_event::{ct_event, ConsumedEvent, HandleEvent, Regular};
use rat_focus::{ContainerFlag, FocusBuilder, FocusContainer, Navigation};
use rat_reloc::RelocatableState;
use rat_salsa::timer::TimeOut;
use rat_salsa::{AppContext, AppState, AppWidget, Control, RenderContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::{type_name, Any, TypeId};
use std::cmp::max;
use std::fmt::Debug;
use std::ops::DerefMut;

pub trait WinSalsaWidget<Global, Message, Error>:
    AppWidget<Global, Message, Error, State = dyn WinSalsaState<Global, Message, Error>>
where
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
}

pub trait WinSalsaState<Global, Message, Error>:
    AppState<Global, Message, Error> + RelocatableState + HasScreenCursor + Any
where
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
    /// Cast the window as a FocusContainer for focus handling.
    fn as_focus_container(&self) -> &dyn FocusContainer;
}

impl<Global, Message, Error> dyn WinSalsaState<Global, Message, Error>
where
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
{
    /// Call the closure for a given window.
    pub fn for_ref<S: WinSalsaState<Global, Message, Error>>(&self, f: impl FnOnce(&S)) {
        let downcast = self.downcast_ref::<S>().expect(type_name::<S>());
        f(downcast)
    }

    /// Call the closure for a given window.
    pub fn for_mut<S: WinSalsaState<Global, Message, Error>>(&mut self, f: impl FnOnce(&mut S)) {
        let downcast = self.downcast_mut::<S>().expect(type_name::<S>());
        f(downcast)
    }

    /// down cast Any style.
    pub fn downcast_ref<R: WinSalsaState<Global, Message, Error>>(&self) -> Option<&R> {
        if self.type_id() == TypeId::of::<R>() {
            let p: *const dyn WinSalsaState<Global, Message, Error> = self;
            Some(unsafe { &*(p as *const R) })
        } else {
            None
        }
    }

    /// down cast Any style.
    pub fn downcast_mut<R: WinSalsaState<Global, Message, Error>>(
        &'_ mut self,
    ) -> Option<&'_ mut R> {
        if (*self).type_id() == TypeId::of::<R>() {
            let p: *mut dyn WinSalsaState<Global, Message, Error> = self;
            Some(unsafe { &mut *(p as *mut R) })
        } else {
            None
        }
    }
}

impl<'a, M: WindowManager, Global, Message, Error> AppWidget<Global, Message, Error>
    for Windows<'a, dyn WinSalsaState<Global, Message, Error>, M>
where
    Global: 'static,
    Message: 'static + Send,
    Error: 'static + Send,
    M: WindowManager + Debug,
    M::Outcome: ConsumedEvent + Into<WindowsOutcome>,
    M::State: HandleEvent<crossterm::event::Event, Regular, M::Outcome>,
{
    type State = WindowsState<
        dyn WinSalsaWidget<Global, Message, Error>,
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
        render_windows(
            self,
            |window, widget_area, buf, window_state| {
                window.render(widget_area, buf, window_state, ctx)
            },
            area,
            buf,
            state,
        )
    }
}

impl<Global, Message, Error, M> HasScreenCursor
    for WindowsState<
        dyn WinSalsaWidget<Global, Message, Error>,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >
where
    M: WindowManager,
    Message: 'static + Send,
    Error: 'static + Send,
{
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

impl<Global, Message, Error, M> FocusContainer
    for WindowsState<
        dyn WinSalsaWidget<Global, Message, Error>,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >
where
    M: WindowManager,
    Message: 'static + Send,
    Error: 'static + Send,
{
    fn build(&self, builder: &mut FocusBuilder) {
        // only have the windows themselves.
        let manager = self.rc.manager.borrow();

        if manager.mode() == WindowMode::Config {
            for handle in self.handles_create() {
                let frame = manager.window_frame(handle);
                // only add the window as widget
                builder.widget(frame.as_has_focus());
            }
        } else if manager.mode() == WindowMode::Widget {
            builder.add_widget(manager.focus(), manager.area(), 0, Navigation::Mouse);
        } else {
            for handle in self.handles_create().into_iter() {
                let frame = manager.window_frame(handle);
                let frame_container = frame.as_focus_container();
                let window_state = self.window_state(handle);

                let tag = builder.start(
                    frame_container.container(),
                    frame_container.area(),
                    frame_container.area_z(),
                );
                builder.container(window_state.borrow().as_focus_container());
                builder.end(tag);
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

impl<Global, Message, Error, M> AppState<Global, Message, Error>
    for WindowsState<
        dyn WinSalsaWidget<Global, Message, Error>,
        dyn WinSalsaState<Global, Message, Error>,
        M,
    >
where
    M: WindowManager,
    M::Outcome: ConsumedEvent + Into<WindowsOutcome>,
    M::State: HandleEvent<crossterm::event::Event, Regular, M::Outcome>,
    Message: 'static + Send,
    Error: 'static + Send,
{
    fn init(&mut self, ctx: &mut AppContext<'_, Global, Message, Error>) -> Result<(), Error> {
        for handle in self.handles_render().into_iter().rev() {
            self.run_for_window(handle, &mut |_window, window_state| window_state.init(ctx))?;
        }
        Ok(())
    }

    fn timer(
        &mut self,
        event: &TimeOut,
        ctx: &mut AppContext<'_, Global, Message, Error>,
    ) -> Result<Control<Message>, Error> {
        for handle in self.handles_render().into_iter().rev() {
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
        // Special action for focus.
        self.rc.manager.borrow_mut().focus_to_front();

        // forward to window-manager
        let r0 = self
            .rc
            .manager
            .borrow_mut()
            .deref_mut()
            .handle(event, Regular)
            .into();

        let r0: Control<Message> = r0.into();

        let r0 = 'k: {
            if !r0.is_consumed() {
                match event {
                    ct_event!(keycode press F(3)) => {
                        let mut manager = self.rc.manager.borrow_mut();
                        let Some(front) = manager.front_window() else {
                            break 'k Control::<Message>::Continue;
                        };

                        let handles = self.handles_create();

                        let mut cur_idx = 0;
                        for (idx, handle) in self.handles_create().iter().enumerate() {
                            if front == *handle {
                                cur_idx = idx;
                            }
                        }

                        cur_idx += 1;
                        if cur_idx >= handles.len() {
                            cur_idx = 0;
                        }

                        let new_front = handles[cur_idx];
                        manager.window_to_front(new_front);

                        let old_focus = manager.focus_focus().take();
                        let frame = manager.window_frame(new_front);
                        let frame_container = frame.as_focus_container();
                        let window_state = self.window_state(new_front);

                        let mut builder = FocusBuilder::new(old_focus);
                        let tag = builder.start(
                            frame_container.container(),
                            frame_container.area(),
                            frame_container.area_z(),
                        );
                        builder.container(window_state.borrow().as_focus_container());
                        builder.end(tag);
                        let focus = builder.build();

                        focus.enable_log();
                        focus.first();
                        manager.set_focus_focus(Some(focus));

                        Control::Changed
                    }
                    ct_event!(keycode press F(4)) => Control::Continue,
                    _ => Control::Continue,
                }
            } else {
                r0
            }
        };

        // forward to all windows
        let r1 = if !r0.is_consumed() {
            'f: {
                for handle in self.handles_render().into_iter().rev() {
                    let r = self.run_for_window(handle, &mut |_window, window_state| {
                        window_state.crossterm(event, ctx)
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
        for handle in self.handles_render().into_iter().rev() {
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
