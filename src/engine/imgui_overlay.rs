use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use cgmath::Vector3;
use imgui::{Condition, Context};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use wgpu::{RenderPass, TextureFormat};
use winit::event::Event;
use winit::window::Window;

use crate::engine::rendering::{RenderCtx, Renderer2D};

pub struct ImguiOverlay {
    render_ctx: Rc<RefCell<RenderCtx>>,
    imgui: Context,
    platform: WinitPlatform,
    renderer: Renderer,

    last_fps_counts: VecDeque<f32>,
}

impl ImguiOverlay {
    pub fn new(render_ctx: Rc<RefCell<RenderCtx>>, window: &winit::window::Window) -> Self {
        let mut imgui = imgui::Context::create();
        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
        imgui.set_ini_filename(None);
        let hidpi_factor = window.scale_factor();
        let font_size = (20.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        let renderer = Renderer::new(
            &mut imgui,
            &render_ctx.borrow().device,
            &render_ctx.borrow().queue,
            RendererConfig {
                texture_format: render_ctx.borrow().surface_config.format,
                depth_format: Some(TextureFormat::Depth32Float),
                ..Default::default()
            },
        );

        Self {
            imgui,
            platform,
            renderer,
            render_ctx,
            last_fps_counts: VecDeque::with_capacity(60),
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>, window: &Window) {
        self.platform
            .handle_event(self.imgui.io_mut(), window, event)
    }

    pub fn prepare_render(&mut self, window: &Window, stats: PerFrameStats) {
        if self.last_fps_counts.len() == self.last_fps_counts.capacity() {
            self.last_fps_counts.pop_front();
        }
        self.last_fps_counts.push_back(stats.fps);
        let average_fps: f32 = self.last_fps_counts.iter().sum::<f32>() / (self.last_fps_counts.len() as f32);

        self.platform
            .prepare_frame(self.imgui.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = self.imgui.frame();

        {
            let window = ui.window("Debug Information");
            window
                .always_auto_resize(true)
                .position([0.0, 0.0], Condition::FirstUseEver)
                .collapsible(false)
                .no_decoration()
                .movable(false)
                .draw_background(false)
                .build(|| {
                    ui.text(format!("FPS: {:.1} ({:.2}ms)", average_fps, 1000.0 / average_fps));
                    ui.text(format!("V: {}  T: {}", stats.num_vertices, stats.num_triangles));
                    ui.text(format!("{:?}", stats.position));
                    ui.text(format!("Chunks: {}", stats.num_chunks));
                });
        }

        self.platform.prepare_render(ui, window);
    }
}

impl Renderer2D for ImguiOverlay {
    fn render<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.renderer
            .render(
                self.imgui.render(),
                &self.render_ctx.borrow().queue,
                &self.render_ctx.borrow().device,
                render_pass,
            )
            .expect("Rendering failed");
    }
}

#[derive(Debug, Clone)]
pub struct PerFrameStats {
    pub fps: f32,
    pub last_frame_time: f32,
    pub num_chunks: u32,
    pub num_vertices: u32,
    pub num_triangles: u32,
    pub position: Vector3<f32>,
}
