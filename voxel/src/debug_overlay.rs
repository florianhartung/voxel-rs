use std::borrow::Borrow;
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;

use cgmath::Vector3;
use egui::{ClippedPrimitive, CollapsingHeader, CollapsingResponse, Color32, Context, Slider, Ui, ViewportId, Visuals, WidgetText};
use egui_wgpu::ScreenDescriptor;
use wgpu::{BindGroup, CommandEncoder, RenderPass, TextureFormat};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::renderer::{RenderCtx, Renderer};
use crate::timing::TimerManager;

pub struct DebugOverlay {
    winit_state: egui_winit::State,
    context: Context,
    renderer: egui_wgpu::Renderer,
    screen_descriptor: ScreenDescriptor,
    render_ctx: Arc<RenderCtx>,

    paint_jobs: Option<Vec<ClippedPrimitive>>,

    last_fps_counts: VecDeque<f32>,
    pub render_distance: i32,
    pub render_empty_chunks: bool,
    pub no_clip: bool,

    output: Option<egui::FullOutput>,
}

impl DebugOverlay {
    pub fn new(render_ctx: Arc<RenderCtx>, window: &Window) -> Self {
        let context = Context::default();
        let winit_state = egui_winit::State::new(context.clone(), context.viewport_id(), window, None, None, None);

        let render_pass = egui_wgpu::Renderer::new(
            &render_ctx.device,
            render_ctx
                .surface_config
                .try_lock()
                .expect("i hope this isn't locked")
                .format,
            Some(TextureFormat::Depth32Float),
            1,
            false,
        );

        let screen_descriptor = ScreenDescriptor {
            pixels_per_point: window.scale_factor() as f32,
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
        };

        Self {
            winit_state,
            context,
            renderer: render_pass,
            screen_descriptor,
            last_fps_counts: VecDeque::with_capacity(10),
            render_distance: 12,
            render_empty_chunks: false,
            no_clip: true,
            render_ctx,
            paint_jobs: None,
            output: None,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let result = self.winit_state.on_window_event(window, event);

        if let WindowEvent::Resized(new_size) = &event {
            self.screen_descriptor.size_in_pixels = [new_size.width, new_size.height];
        }

        self.screen_descriptor.pixels_per_point = window.scale_factor() as f32;

        result.consumed
    }

    pub fn build_ui(&mut self, window: &Window, stats: PerFrameStats, timer: &mut TimerManager) {
        if self.last_fps_counts.len() == self.last_fps_counts.capacity() {
            self.last_fps_counts.pop_front();
        }
        self.last_fps_counts.push_back(stats.fps);
        let average_fps: f32 = self.last_fps_counts.iter().sum::<f32>() / (self.last_fps_counts.len() as f32);

        self.context
            .begin_pass(self.winit_state.take_egui_input(window));

        self.context.set_visuals(Visuals {
            window_fill: Color32::TRANSPARENT,
            panel_fill: Color32::TRANSPARENT,
            override_text_color: Some(Color32::RED),
            faint_bg_color: Color32::RED,
            extreme_bg_color: Color32::BLUE,
            ..Default::default()
        });

        egui::CentralPanel::default().show(&self.context, |ui| {
            ui.collapsing_opened("General", |ui| {
                ui.label(format!("FPS: {:.1} ({:.2}ms)", average_fps, 1000.0 / average_fps));
                ui.label(format!("Location: {:?}", stats.position));
                ui.checkbox(&mut self.no_clip, "noclip");
            });

            ui.collapsing_opened("Memory", |ui| {
                ui.label(format!("Voxel data: {}MB", stats.total_voxel_data_size / 2_i32.pow(20) as usize));
                ui.label(format!("Mesh data: {}MB", stats.total_mesh_data_size / 2_i32.pow(20) as usize));
            });

            ui.collapsing_opened("World generation", |ui| {
                ui.label(format!("Total chunks: {}", stats.num_chunks));
                ui.label(format!("Worker thread pool queue size: {}", stats.worker_thread_pool_queue_size));
                ui.label(format!(
                    "Generated pending chunk queue size: {}",
                    stats.current_chunkdata_buffer_size
                ));
                ui.label(format!("Current meshed chunks queue size (waiting to be saved): {}", stats.current_meshed_chunks_queue_size));
            });

            ui.collapsing_opened("Rendering", |ui| {
                ui.add(Slider::new(&mut self.render_distance, 1..=32).text("Render distance"));
                ui.label(format!(
                    "Currently rendered chunk radius: {}",
                    stats.currently_rendered_chunk_radius
                ));
                ui.label(format!("V: {}  T: {}", stats.num_vertices, stats.num_triangles));
                ui.checkbox(&mut self.render_empty_chunks, "render empty chunks");
            });

            ui.collapsing("Timing", |ui| {
                timer
                    .get_all()
                    .iter()
                    .for_each(|(name, duration_sec)| {
                        ui.label(format!("{}: {:.2}ms", name, duration_sec * 1000.0));
                    });
                timer.clear();
            });
        });
        self.output = Some(self.context.end_frame());
    }

    /// Must be called before rendering this overlay.
    /// This will tessellate the ui and upload all resources to the gpu
    pub fn prepare_render(&mut self, command_encoder: &mut CommandEncoder) {
        let full_output = mem::take(&mut self.output).expect("Failed to get output of egui preparation result");

        let paint_jobs = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let tdelta = full_output.textures_delta;

        for (t_id, tdelta) in tdelta.set {
            self.renderer
                .update_texture(&self.render_ctx.device, &self.render_ctx.queue, t_id, &tdelta);
        }

        self.renderer.update_buffers(
            &self.render_ctx.device,
            &self.render_ctx.queue,
            command_encoder,
            &paint_jobs,
            &self.screen_descriptor,
        );

        self.paint_jobs = Some(paint_jobs);
    }
}

impl Renderer for DebugOverlay {
    fn render<'a>(&'a self, render_pass: RenderPass<'a>, _camera_bind_group: &'a BindGroup, render_ctx: &RenderCtx) {
        let paint_jobs = self
            .paint_jobs
            .as_ref()
            .expect("no paint jobs were prepared");

        self.renderer
            .render(&mut render_pass.forget_lifetime(), paint_jobs, &self.screen_descriptor);
    }
}

#[derive(Debug, Clone)]
pub struct PerFrameStats {
    pub fps: f32,
    pub last_frame_time: f32,
    pub num_chunks: u32,
    pub num_vertices: usize,
    pub num_triangles: usize,
    pub position: Vector3<f32>,
    pub total_voxel_data_size: usize,
    pub total_mesh_data_size: usize,
    pub currently_rendered_chunk_radius: i32,
    pub current_meshed_chunks_queue_size: usize,
    pub worker_thread_pool_queue_size: usize,
    pub current_chunkdata_buffer_size: usize,
}

trait CollapsingOpened {
    fn collapsing_opened<R>(&mut self, heading: impl Into<WidgetText>, add_contents: impl FnOnce(&mut Ui) -> R) -> CollapsingResponse<R>;
}

impl CollapsingOpened for Ui {
    fn collapsing_opened<R>(&mut self, heading: impl Into<WidgetText>, add_contents: impl FnOnce(&mut Ui) -> R) -> CollapsingResponse<R> {
        CollapsingHeader::new(heading)
            .default_open(true)
            .show(self, add_contents)
    }
}
