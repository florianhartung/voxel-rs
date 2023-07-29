use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use wgpu::{include_wgsl, vertex_attr_array};

use crate::engine::rendering::texture::Texture;
use crate::engine::rendering::{RenderCtx, Renderer};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    renderer: MeshRenderer,
}

impl Debug for Mesh {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mesh {{renderer: {:?}", self.renderer)
    }
}

impl Mesh {
    pub fn new(
        render_ctx: Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
    ) -> Self {
        let mesh_render = MeshRenderer::new(render_ctx, camera_bind_group_layout, &vertices, &indices);

        Self {
            vertices,
            indices,
            renderer: mesh_render,
        }
    }

    pub fn update(&mut self, new_vertices: Vec<Vertex>, new_indices: Vec<u32>) {
        self.vertices = new_vertices;
        self.indices = new_indices;

        self.renderer
            .update(&self.vertices, &self.indices);
    }

    pub fn get_renderer(&self) -> &MeshRenderer {
        &self.renderer
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
    direction: Vector3<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            position,
            color,
            direction,
        }
    }

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            attributes: &ATTRIBUTES,
            step_mode: wgpu::VertexStepMode::Vertex,
        }
    }
}

#[derive(Debug)]
pub struct MeshRenderer {
    render_ctx: Rc<RefCell<RenderCtx>>,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl MeshRenderer {
    pub fn new(
        render_ctx: Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        vertices: &Vec<Vertex>,
        indices: &Vec<u32>,
    ) -> Self {
        let ctx = render_ctx.borrow();

        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh vertex buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(vertices.as_slice()),
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh index buffer"),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(indices.as_slice()),
            });

        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh render pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[camera_bind_group_layout],
            });

        let render_pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default render pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    buffers: &[Vertex::layout()],
                    entry_point: "vs_main",
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    entry_point: "fs_main",
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Back),
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: Default::default(),
                multiview: None,
            });
        drop(ctx);

        Self {
            render_ctx,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            render_pipeline,
        }
    }

    pub fn update(&mut self, _new_vertices: &Vec<Vertex>, _new_indices: &Vec<u32>) {
        todo!("Update buffers")
    }
}

impl Renderer for MeshRenderer {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup) {
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.set_bind_group(0, camera_bind_group, &[]);

        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

// TODO
struct MeshBuilder {}
