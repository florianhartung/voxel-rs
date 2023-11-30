use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Mutex;

use wgpu::{PresentMode, StoreOp, TextureFormat};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::rendering::camera::Camera;
use crate::rendering::texture::Texture;

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

#[derive(Debug)]
pub struct RenderCtx {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: Mutex<wgpu::SurfaceConfiguration>,
    depth_texture: Mutex<Texture>,
}

impl RenderCtx {
    pub async fn new(window: &Window, enable_vsync: bool) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // # Safety
        // The surface needs to live as long as the window that created it.
        // This is safe because RenderState owns both
        let surface = unsafe { instance.create_surface(&window) }.expect("WGPU failed to create a surface from the window");

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
            .find(TextureFormat::is_srgb)
            .expect("Could not find a surface capability that supports sRGB");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: if enable_vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: Vec::new(),
        };

        surface.configure(&device, &surface_config);

        let depth_texture = Texture::new_depth_texture(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config: Mutex::new(surface_config),
            depth_texture: Mutex::new(depth_texture),
        }
    }

    pub fn resize(&self, new_size: &PhysicalSize<u32>) {
        assert!(new_size.width > 0 && new_size.height > 0, "Window size must be greater than zero");

        let mut surface_config = self.surface_config.try_lock().expect("aa");
        surface_config.width = new_size.width;
        surface_config.height = new_size.height;

        self.surface
            .configure(&self.device, &*surface_config);

        let mut depth_texture = self
            .depth_texture
            .try_lock()
            .expect("The depth texture is only locked by this function and while rendering");
        *depth_texture = Texture::new_depth_texture(&self.device, &*surface_config);
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
            render_ctx: self,
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

        let depth_texture = &self
            .render_ctx
            .depth_texture
            .try_lock()
            .expect("Mutex to be unlocked");

        let mut render_pass = self
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target_texture_view,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: StoreOp::Store,
                    },
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_load_op,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        self.clear_before_next_render = false;

        renderer.render(&mut render_pass, &camera.bind_group);
    }

    pub fn render2d<T: Renderer2D>(&mut self, renderer: &mut T) {
        renderer.prepare(&mut self.encoder);

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

        let depth_texture = &self
            .render_ctx
            .depth_texture
            .try_lock()
            .expect("Mutex to be unlocked");

        let mut render_pass = self
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target_texture_view,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: StoreOp::Store,
                    },
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_load_op,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        self.clear_before_next_render = false;

        renderer.render(&mut render_pass);
        drop(render_pass)
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

pub trait Renderer2D {
    fn prepare(&mut self, _: &mut wgpu::CommandEncoder);

    fn render<'a: 'b + 'c, 'b, 'c>(&'a mut self, render_pass: &'b mut wgpu::RenderPass<'c>);
}
