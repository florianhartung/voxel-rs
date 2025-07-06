//! # A wrapper for the new dumb winit architecture

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

pub trait Application<S>: Sized {
    fn new(window: Arc<Window>, initial_window_size: (u32, u32), init_state: S) -> Self;
    fn handle_event(&mut self, event: Event, active_event_loop: &ActiveEventLoop);
}

pub enum Event {
    DeviceEvent(winit::event::DeviceEvent),
    WindowEvent(winit::event::WindowEvent),
}

/// This makes winit fun to use again for simple single-window applications
pub fn run_window_app<T: Application<S>, S>(init_state: S) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut wtf = Wtf::<T, S>::new(init_state);
    event_loop.run_app(&mut wtf).unwrap()
}

struct Wtf<T, S> {
    window_state: Option<(Arc<Window>, T)>,
    init_state: Option<S>,
}
impl<T, S> Wtf<T, S> {
    pub fn new(init_state: S) -> Self {
        Self {
            window_state: None,
            init_state: Some(init_state),
        }
    }
}

impl<S, T: Application<S>> ApplicationHandler for Wtf<T, S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();
        let window = Arc::new(window);

        let state = T::new(
            window.clone(),
            window.inner_size().into(),
            self.init_state
                .take()
                .expect("only one window and thus always an init state to exist"),
        );

        self.window_state = Some((window, state));

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        let (_window, state) = self.window_state.as_mut().unwrap();

        state.handle_event(Event::DeviceEvent(event), event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let (window, state) = self.window_state.as_mut().unwrap();
        debug_assert_eq!(window_id, window.id(), "there can only be one window");

        state.handle_event(Event::WindowEvent(event), event_loop);
    }
}
