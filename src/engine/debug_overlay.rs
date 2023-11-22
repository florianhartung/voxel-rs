use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::rc::Rc;

use cgmath::Vector3;
use egui::{ClippedPrimitive, CollapsingHeader, CollapsingResponse, Color32, Context, Slider, Ui, Visuals, WidgetText};
use egui_wgpu::renderer::ScreenDescriptor;
use wgpu::TextureFormat::Depth32Float;
use wgpu::{CommandEncoder, RenderPass};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::engine::rendering::{RenderCtx, Renderer2D};
use crate::engine::timing::TimerManager;

pub struct DebugOverlay {
    winit_state: egui_winit::State,
    context: Context,
    renderer: egui_wgpu::Renderer,
    screen_descriptor: ScreenDescriptor,
    render_ctx: Rc<RefCell<RenderCtx>>,

    paint_jobs: Option<Vec<ClippedPrimitive>>,

    last_fps_counts: VecDeque<f32>,
    pub render_distance: i32,
    pub render_empty_chunks: bool,
    pub no_clip: bool,
}

impl DebugOverlay {
    pub fn new(render_ctx: Rc<RefCell<RenderCtx>>, window: &Window) -> Self {
        let winit_state = egui_winit::State::new(window);
        let context = Context::default();
        let render_pass = egui_wgpu::Renderer::new(
            &render_ctx.borrow().device,
            render_ctx.borrow().surface_config.format,
            Some(Depth32Float),
            1,
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
            last_fps_counts: VecDeque::with_capacity(240),
            render_distance: 8,
            render_empty_chunks: false,
            no_clip: true,
            render_ctx,
            paint_jobs: None,
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        let result = self.winit_state.on_event(&self.context, event);

        if let WindowEvent::Resized(new_size) = &event {
            self.screen_descriptor.size_in_pixels = [new_size.width, new_size.height];
        }

        self.screen_descriptor.pixels_per_point = self.winit_state.pixels_per_point();

        result.consumed
    }

    pub fn prepare_render<'a>(&'a mut self, window: &Window, stats: PerFrameStats, timer: &mut TimerManager) -> OverlayRenderer<'a> {
        if self.last_fps_counts.len() == self.last_fps_counts.capacity() {
            self.last_fps_counts.pop_front();
        }
        self.last_fps_counts.push_back(stats.fps);
        let average_fps: f32 = self.last_fps_counts.iter().sum::<f32>() / (self.last_fps_counts.len() as f32);

        self.context
            .begin_frame(self.winit_state.take_egui_input(window));

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

            ui.collapsing_opened("Rendering", |ui| {
                ui.add(Slider::new(&mut self.render_distance, 1..=32).text("Render distance"));
                ui.label(format!(
                    "Currently rendered chunk radius: {}",
                    stats.currently_rendered_chunk_radius
                ));
                ui.label(format!("V: {}  T: {}", stats.num_vertices, stats.num_triangles));
                ui.label(format!("Chunks: {}", stats.num_chunks));
                ui.checkbox(&mut self.render_empty_chunks, "render empty chunks");
                ui.label(format!("Chunk gen queue size: {}", stats.current_datagen_queue_size));
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
        OverlayRenderer {
            output: Some(self.context.end_frame()),
            parent: self,
        }
    }
}

pub struct OverlayRenderer<'a> {
    output: Option<egui::FullOutput>,
    parent: &'a mut DebugOverlay,
}

impl Renderer2D for OverlayRenderer<'_> {
    fn prepare(&mut self, command_encoder: &mut CommandEncoder) {
        let full_output = std::mem::take(&mut self.output).expect("Failed to get output of egui preparation result");

        let paint_jobs = self.parent.context.tessellate(full_output.shapes);
        let tdelta = full_output.textures_delta;

        let render_ctx = self.parent.render_ctx.borrow();
        for (t_id, tdelta) in tdelta.set {
            self.parent
                .renderer
                .update_texture(&render_ctx.device, &render_ctx.queue, t_id, &tdelta);
        }

        self.parent.renderer.update_buffers(
            &render_ctx.device,
            &render_ctx.queue,
            command_encoder,
            &paint_jobs,
            &self.parent.screen_descriptor,
        );

        self.parent.paint_jobs = Some(paint_jobs);
    }

    fn render<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        let paint_jobs = mem::take(&mut self.parent.paint_jobs).expect("no paint jobs were prepared");

        self.parent
            .renderer
            .render(render_pass, &paint_jobs, &self.parent.screen_descriptor);
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
    pub current_datagen_queue_size: usize,
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
