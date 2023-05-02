use cgmath::Deg;
use winit::dpi::PhysicalSize;
use winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub use starter::start;

use crate::engine::frame_timer::FrameTimer;
use crate::engine::rendering::camera::{Camera, CameraController};
use crate::engine::rendering::RenderCtx;
use crate::engine::world::chunk::renderer::ChunkRenderer;
use crate::engine::world::chunk::{Chunk, MeshedChunk};

#[macro_use]
mod macros;
mod frame_timer;
mod rendering;
mod starter;
mod world;

pub struct Engine {
    window: Window,
    frame_timer: FrameTimer,
    render_ctx: RenderCtx,

    chunk: MeshedChunk,
    chunk_renderer: ChunkRenderer,
    chunk2: MeshedChunk,
    chunk_renderer2: ChunkRenderer,

    camera: Camera,
    camera_controller: CameraController,
    mouse_pressed: bool,
}

impl Engine {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let window = create_basic_window(event_loop);
        let render_ctx = pollster::block_on(RenderCtx::new(&window));

        let camera = Camera::new(
            &render_ctx,
            (0.0, 5.0, 10.0),
            Deg(-90.0),
            Deg(-20.0),
            render_ctx.surface_config.width,
            render_ctx.surface_config.height,
            Deg(45.0),
            0.1,
            1000.0,
        );

        let chunk = Chunk::new_with_random_data().into_meshed();
        let chunk_renderer = chunk.get_renderer(&render_ctx, &camera.bind_group_layout);

        let chunk2 = Chunk::new_with_random_data().into_meshed();
        let chunk_renderer2 = chunk.get_renderer(&render_ctx, &camera.bind_group_layout);

        Self {
            window,
            frame_timer: FrameTimer::new(),
            render_ctx,
            chunk,
            chunk_renderer,
            chunk2,
            chunk_renderer2,
            camera,
            camera_controller: CameraController::new(30.0, 0.5),
            mouse_pressed: false,
        }
    }

    fn render(&mut self) {
        let dt = self.frame_timer.get_dt();

        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera.update_buffer(&self.render_ctx);

        let mut handle = self.render_ctx.start_rendering();
        handle.render(&self.chunk_renderer, &self.camera);
        handle.finish_rendering();

        println!("{:.2}fps", 1.0 / dt.as_secs_f32());
    }

    fn handle_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if self.handle_resize(&event) {
            return;
        }

        match event {
            key_press!(VirtualKeyCode::Escape) | close_requested!() => {
                *control_flow = ControlFlow::ExitWithCode(0)
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(virtual_keycode),
                                state,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                self.camera_controller
                    .process_keyboard(&virtual_keycode, &state);
            }

            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    },
                ..
            } => {
                self.chunk.randomize_data();
                self.chunk
                    .update_renderer(&mut self.chunk_renderer, &self.render_ctx);
                self.mouse_pressed = matches!(state, ElementState::Pressed);
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(delta.0, delta.1);
                }
            }
            _ => {}
        }
    }

    fn handle_resize(&mut self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    self.render_ctx.resize(&new_size);
                    self.camera.resize(new_size.width, new_size.height);
                    true
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    self.render_ctx.resize(new_inner_size);
                    self.camera
                        .resize(new_inner_size.width, new_inner_size.height);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

fn create_basic_window(event_loop: &EventLoop<()>) -> Window {
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    window
}
