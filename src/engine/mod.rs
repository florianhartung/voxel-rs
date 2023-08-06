use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Deg, EuclideanSpace};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub use starter::start;

use crate::engine::frame_timer::FrameTimer;
use crate::engine::imgui_overlay::{ImguiOverlay, PerFrameStats};
use crate::engine::rendering::camera::{Camera, CameraController};
use crate::engine::rendering::RenderCtx;
use crate::engine::timing::TimerManager;
use crate::engine::world::chunk_manager::ChunkManager;

#[macro_use]
mod macros;
mod frame_timer;
mod imgui_overlay;
mod rendering;
mod starter;
mod timing;
pub(crate) mod util;
pub mod vector_utils;
pub mod world;

pub struct Engine {
    window: Window,
    frame_timer: FrameTimer,
    render_ctx: Rc<RefCell<RenderCtx>>,

    chunk_manager: ChunkManager,

    camera: Camera,
    camera_controller: CameraController,
    mouse_pressed: bool,

    imgui_overlay: ImguiOverlay,
    timer: TimerManager,
}

impl Engine {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let window = create_basic_window(event_loop);
        let render_ctx = Rc::new(RefCell::new(pollster::block_on(RenderCtx::new(&window))));

        let camera = Camera::new(
            &render_ctx.borrow(),
            (-79.21167, 5.4288225, -39.484493),
            Deg(-42.0),
            Deg(-20.0),
            render_ctx.borrow().surface_config.width,
            render_ctx.borrow().surface_config.height,
            Deg(45.0),
            0.1,
            1000.0,
        );

        let mut chunk_manager = ChunkManager::new(camera.position.to_vec());
        chunk_manager.generate_chunks();
        chunk_manager.generate_chunk_meshes(&render_ctx, &camera.bind_group_layout);

        let imgui_overlay = ImguiOverlay::new(render_ctx.clone(), &window);

        let mut timer = TimerManager::new();
        timer.start("frame");

        Self {
            window,
            frame_timer: FrameTimer::new(),
            render_ctx,
            camera,
            camera_controller: CameraController::new(20.0, 0.5),
            mouse_pressed: false,
            chunk_manager,
            imgui_overlay,
            timer,
        }
    }

    fn render(&mut self) {
        let render_ctx = self.render_ctx.borrow();

        let dt = self.frame_timer.get_dt();

        self.chunk_manager.render_distance = self.imgui_overlay.render_distance;
        self.chunk_manager.render_empty_chunks = self.imgui_overlay.render_empty_chunks;

        self.timer.start("update_camera");
        self.camera_controller
            .update_camera(&mut self.camera, dt);
        self.camera.update_buffer(&render_ctx);
        self.timer.end("update_camera");

        self.timer.start("chunk_manager");
        self.chunk_manager
            .update_player_location(self.camera.position.to_vec());

        self.timer.start("chunk_manager_generate");
        self.chunk_manager.generate_chunks();
        self.timer.end("chunk_manager_generate");

        self.timer.start("chunk_manager_meshing");
        self.chunk_manager
            .generate_chunk_meshes(&self.render_ctx, &self.camera.bind_group_layout);
        self.timer.end("chunk_manager_meshing");

        self.timer.start("chunk_manager_unloading");
        self.chunk_manager.unload_chunks();
        self.timer.end("chunk_manager_unloading");
        self.timer.end("chunk_manager");

        let stats = PerFrameStats {
            fps: 1.0 / dt.as_secs_f32(),
            last_frame_time: dt.as_secs_f32() * 1000.0,
            position: self.camera.position.to_vec(),
            num_chunks: self.chunk_manager.chunks.len() as u32,
            num_vertices: self.chunk_manager.total_vertices,
            num_triangles: self.chunk_manager.total_triangles,
            total_voxel_data_size: self.chunk_manager.total_voxel_data_size,
            total_mesh_data_size: self.chunk_manager.total_mesh_data_size,
            currently_rendered_chunk_radius: self.chunk_manager.current_chunk_mesh_radius - 1,
        };

        self.timer.start("imgui_prepare");
        self.imgui_overlay
            .prepare_render(&self.window, stats, &mut self.timer);
        self.timer.end("imgui_prepare");

        let mut handle = render_ctx.start_rendering();
        self.timer.start("render_3d");
        handle.render(&self.chunk_manager, &self.camera);
        self.timer.end("render_3d");

        self.timer.start("render_ui");
        handle.render2d(&mut self.imgui_overlay);
        self.timer.end("render_ui");

        self.timer.start("render_final");
        handle.finish_rendering();
        self.timer.end("render_final");
    }

    fn handle_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if self.handle_resize(&event) {
            self.imgui_overlay
                .handle_event(&event, &self.window);
            return;
        }

        match event {
            key_press!(VirtualKeyCode::Escape) | close_requested!() => *control_flow = ControlFlow::ExitWithCode(0),
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
                // self.chunk.randomize_data();
                // self.chunk
                //     .update_renderer(&mut self.chunk_renderer, &self.render_ctx);
                self.mouse_pressed = matches!(state, ElementState::Pressed);
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.mouse_pressed {
                    self.camera_controller
                        .process_mouse(delta.0, delta.1);
                }
            }
            _ => {}
        }

        self.imgui_overlay
            .handle_event(&event, &self.window);
    }

    fn handle_resize(&mut self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    self.render_ctx.borrow_mut().resize(new_size);
                    self.camera
                        .resize(new_size.width, new_size.height);
                    true
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    self.render_ctx
                        .borrow_mut()
                        .resize(new_inner_size);
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
    WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(800, 600))
        .build(event_loop)
        .unwrap()
}
