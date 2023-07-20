use std::mem::ManuallyDrop;

use wgpu::TextureFormat;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::engine::rendering::camera::Camera;
use crate::engine::rendering::texture::Texture;

pub mod camera;
pub mod texture;

pub trait HasBufferLayout {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub struct RenderHandle<'a> {
    render_ctx: &'a RenderCtx,
    encoder: ManuallyDrop<wgpu::CommandEncoder>,
    target_texture: ManuallyDrop<wgpu::SurfaceTexture>,
    target_texture_view: wgpu::TextureView,
    clear_before_next_render: bool,
}

pub struct RenderCtx {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    depth_texture: Texture,
}

impl RenderCtx {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // # Safety
        // The surface needs to live as long as the window that created it.
        // This is safe because RenderState owns both
        let surface = unsafe { instance.create_surface(&window) }
            .expect("WGPU failed to create a surface from the window");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("WGPU could not find a compatible adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Could not request device and queue");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(TextureFormat::is_srgb)
            .next()
            .expect("Could not find a surface capability that supports sRGB");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: Vec::new(),
        };

        surface.configure(&device, &surface_config);

        let depth_texture = Texture::new_depth_texture(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config,
            depth_texture,
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        assert!(
            new_size.width > 0 && new_size.height > 0,
            "Window size must be greater than zero"
        );

        (self.surface_config.width, self.surface_config.height) = (new_size.width, new_size.height);
        self.surface.configure(&self.device, &self.surface_config);

        self.depth_texture = Texture::new_depth_texture(&self.device, &self.surface_config);
    }

    pub fn start_rendering(&self) -> RenderHandle {
        let target_texture = self
            .surface
            .get_current_texture()
            .expect("Could not retrieve new texture from surface");

        let target_texture_view = target_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        RenderHandle {
            render_ctx: &self,
            encoder: ManuallyDrop::new(encoder),
            target_texture: ManuallyDrop::new(target_texture),
            target_texture_view,
            clear_before_next_render: true,
        }
    }
}

impl RenderHandle<'_> {
    pub fn render<T: Renderer>(&mut self, renderer: &T, camera: &Camera) {
        let (load_op, depth_load_op) = if self.clear_before_next_render {
            (
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.4941,
                    g: 0.6627,
                    b: 1.0,
                    a: 1.0,
                }),
                wgpu::LoadOp::Clear(1.0),
            )
        } else {
            (wgpu::LoadOp::Load, wgpu::LoadOp::Load)
        };

        let mut render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.target_texture_view,
                ops: wgpu::Operations {
                    load: load_op,
                    store: true,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.render_ctx.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: depth_load_op,
                    store: true,
                }),
                stencil_ops: None,
            }),
        });
        self.clear_before_next_render = false;

        renderer.render(&mut render_pass, &camera.bind_group);
    }
    pub fn finish_rendering(self) {} // Here self is dropped
}

impl Drop for RenderHandle<'_> {
    fn drop(&mut self) {
        let encoder = unsafe { ManuallyDrop::take(&mut self.encoder) };
        let target_texture = unsafe { ManuallyDrop::take(&mut self.target_texture) };

        self.render_ctx
            .queue
            .submit(std::iter::once(encoder.finish()));
        target_texture.present();
    }
}

pub trait Renderer {
    fn render<'a>(&'a self, _: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup);
}
