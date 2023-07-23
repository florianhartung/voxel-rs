use cgmath::{Vector3, Zero};

use crate::engine::rendering::RenderCtx;
use crate::engine::world::mesh::renderer::MeshRenderer;

pub mod renderer;

#[derive(Debug)]
pub struct MeshVertex {
    pub(crate) position: [f32; 3],
    pub color: [f32; 3],
}

impl MeshVertex {
    pub fn new<T: Into<[f32; 3]>>(position: T, color: T) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
        }
    }
}

pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }

    pub fn get_renderer(
        &self,
        render_ctx: &RenderCtx,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        mesh_position: Vector3<f32>,
    ) -> MeshRenderer {
        MeshRenderer::new(render_ctx, &self.vertices, &self.indices, camera_bind_group_layout, mesh_position)
    }

    pub fn update_renderer(&self, mesh_renderer: &mut MeshRenderer, render_ctx: &RenderCtx) {
        mesh_renderer.update_data(&self.vertices, &self.indices, render_ctx);
    }

    /// a, b, c, d counterclockwise
    pub fn add_quad(&mut self, a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>, d: Vector3<f32>, color: Vector3<f32>, is_backface: bool) {
        let new_vertices = [
            MeshVertex::new(a, color),
            MeshVertex::new(b, color),
            MeshVertex::new(c, color),
            MeshVertex::new(d, color),
        ];

        let base_index = self.vertices.len() as u32;

        let indices = if !is_backface { [3, 2, 1, 1, 0, 3] } else { [0, 1, 2, 2, 3, 0] };

        let new_indices = [
            indices[0] + base_index,
            indices[1] + base_index,
            indices[2] + base_index,
            indices[3] + base_index,
            indices[4] + base_index,
            indices[5] + base_index,
        ];

        self.vertices.extend(new_vertices);
        self.indices.extend(new_indices);
    }
}
