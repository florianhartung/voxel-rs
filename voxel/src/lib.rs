use std::sync::Arc;

use cgmath::{Deg, EuclideanSpace};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowBuilder};

pub use starter::start;

use crate::debug_overlay::{DebugOverlay, PerFrameStats};
use crate::frame_timer::FrameTimer;
use crate::rendering::camera::{Camera, CameraController};
use crate::rendering::RenderCtx;
use crate::timing::TimerManager;
use crate::world::chunk_manager::ChunkManager;

#[macro_use]
mod macros;
mod debug_overlay;
mod frame_timer;
mod rendering;
mod starter;
mod timing;
pub(crate) mod util;
pub mod vector_utils;
pub mod world;

pub struct EngineConfig {
    pub run_benchmark: bool,
    pub vsync: bool,
    pub window_size: (u32, u32),
    pub fullscreen: bool,
}

pub struct Engine {
    window: Window,
    frame_timer: FrameTimer,
    render_ctx: Arc<RenderCtx>,

    chunk_manager: ChunkManager,

    camera: Camera,
    camera_controller: CameraController,
    mouse_locked: bool,

    egui_interface: DebugOverlay,
    timer: TimerManager,
}

impl Engine {
    fn new(event_loop: &EventLoop<()>, engine_config: EngineConfig) -> Self {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(engine_config.window_size.0, engine_config.window_size.1))
            .build(event_loop)
            .unwrap();
        if engine_config.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let render_ctx = pollster::block_on(RenderCtx::new(&window, engine_config.vsync));

        let render_ctx = Arc::new(render_ctx);

        let (width, height) = {
            let surface_config = render_ctx
                .surface_config
                .try_lock()
                .expect("AAAAAA");

            (surface_config.width, surface_config.height)
        };

        let camera = Camera::new(
            &*render_ctx,
            (-79.21167, 5.4288225, -39.484493),
            Deg(-42.0),
            Deg(-20.0),
            width,
            height,
            Deg(80.0),
            0.1,
            1000.0,
        );

        let mut timer = TimerManager::new();
        timer.start("frame");

        let mut chunk_manager = ChunkManager::new(camera.position.to_vec(), &render_ctx, &camera.bind_group_layout);
        chunk_manager.generate_chunks(&mut timer);
        chunk_manager.generate_chunk_meshes(&*render_ctx, &camera.bind_group_layout, &mut timer);

        let imgui_overlay = DebugOverlay::new(Arc::clone(&render_ctx), &window);

        Self {
            window,
            frame_timer: FrameTimer::new(),
            render_ctx,
            camera,
            camera_controller: CameraController::new(100.0, 0.5),
            mouse_locked: false,
            chunk_manager,
            egui_interface: imgui_overlay,
            timer,
        }
    }

    fn render(&mut self) {
        self.timer.start("render_all");
        let render_ctx = &*self.render_ctx;

        let dt = self.frame_timer.get_dt();

        self.chunk_manager.render_distance = self.egui_interface.render_distance;
        self.chunk_manager.render_empty_chunks = self.egui_interface.render_empty_chunks;
        self.camera_controller.no_clip = self.egui_interface.no_clip;

        self.timer.start("update_camera");

        self.camera_controller
            .update_physics(&mut self.camera, &self.chunk_manager, dt);

        self.camera_controller
            .update_camera(&mut self.camera, dt);
        self.camera.update_buffer(&render_ctx);
        self.timer.end("update_camera");

        self.timer.start("chunk_manager");
        self.chunk_manager
            .update_player_location(self.camera.position.to_vec());

        self.chunk_manager
            .generate_chunks(&mut self.timer);

        self.chunk_manager
            .generate_chunk_meshes(&*render_ctx, &self.camera.bind_group_layout, &mut self.timer);

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
            current_meshgen_queue_size: self.chunk_manager.chunk_mesh_queue.len(),
            current_chunkgen_queue_size: self.chunk_manager.location_queue.len(),
            current_chunkdata_buffer_size: self.chunk_manager.generated_chunks_queue.len(),
        };

        self.timer.start("imgui_prepare");
        self.egui_interface
            .build_ui(&self.window, stats, &mut self.timer);
        self.timer.end("imgui_prepare");

        let mut handle = render_ctx.start_rendering();

        // Use command encoder to prepare egui
        self.egui_interface
            .prepare_render(handle.get_command_encoder());

        self.timer.start("render_3d");
        handle.render(&self.chunk_manager, &self.camera);
        self.timer.end("render_3d");

        self.timer.start("render_ui");
        handle.render(&mut self.egui_interface, &self.camera);
        self.timer.end("render_ui");

        self.timer.start("render_final");
        handle.finish_rendering();
        self.timer.end("render_final");
        self.timer.end("render_all");
    }

    fn handle_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if self.handle_resize(&event) {
            if let Event::WindowEvent { event, .. } = event {
                self.egui_interface.handle_event(&event);
            }
            return;
        }

        match event {
            key_press!(VirtualKeyCode::Escape) | close_requested!() => *control_flow = ControlFlow::ExitWithCode(0),
            key_press!(VirtualKeyCode::LAlt) => {
                self.mouse_locked = !self.mouse_locked;
                self.window.set_cursor_visible(!self.mouse_locked);
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
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.mouse_locked {
                    self.camera_controller
                        .process_mouse(delta.0, delta.1);
                    self.window
                        .set_cursor_position(get_window_center_position(&self.window))
                        .expect("Could not center mouse");
                }
            }
            _ => {}
        }

        if let Event::WindowEvent { event, .. } = event {
            self.egui_interface.handle_event(&event);
        }
    }

    fn handle_resize(&mut self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    self.render_ctx.resize(new_size);
                    self.camera
                        .resize(new_size.width, new_size.height);
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

fn get_window_center_position(window: &Window) -> PhysicalPosition<u32> {
    let inner_size = window.inner_size();
    PhysicalPosition::new(inner_size.width / 2, inner_size.height / 2)
}
