use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;

use crate::{Engine, EngineConfig};

pub fn start(engine_config: EngineConfig) -> ! {
    let event_loop = EventLoop::new();

    let mut engine = Engine::new(&event_loop, engine_config);

    // Workaround for erroneous first resize winit event on windows
    let mut first_resize_detector = FirstResizeDetector::new();

    event_loop.run(move |event, _, control_flow| {
        if first_resize_detector.check(&event) {
            return;
        }

        match event {
            Event::MainEventsCleared => {
                engine.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                engine.render();
            }
            _ => engine.handle_event(event, control_flow),
        }
    });
}

/**
 * Needed for winit workaround, where first resize is erroneous on windows
 */
struct FirstResizeDetector {
    has_skipped_first_resize: bool,
}

impl FirstResizeDetector {
    pub fn new() -> Self {
        Self {
            has_skipped_first_resize: false,
        }
    }
    #[cfg(target_os = "windows")]
    pub fn check(&mut self, event: &Event<()>) -> bool {
        if self.has_skipped_first_resize {
            return false;
        }

        if let Event::WindowEvent {
            event: WindowEvent::Resized { .. },
            ..
        } = *event
        {
            self.has_skipped_first_resize = true;
            true
        } else {
            false
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn check(&mut self, event: &Event<()>) -> bool {
        false
    }
}
