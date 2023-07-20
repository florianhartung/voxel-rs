use crate::engine::rendering::RenderCtx;
use crate::engine::world::mesh::renderer::MeshRenderer;
use cgmath::{Vector3, Zero};

pub mod renderer;

#[derive(Debug)]
pub struct MeshVertex {
    pub(crate) position: [f32; 3],
}

impl MeshVertex {
    pub fn from_pos<T: Into<[f32; 3]>>(position: T) -> Self {
        Self {
            position: position.into(),
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
        MeshRenderer::new(
            render_ctx,
            &self.vertices,
            &self.indices,
            camera_bind_group_layout,
            mesh_position,
        )
    }

    pub fn update_renderer(&self, mesh_renderer: &mut MeshRenderer, render_ctx: &RenderCtx) {
        mesh_renderer.update_data(&self.vertices, &self.indices, render_ctx);
    }

    /// a, b, c, d counterclockwise
    pub fn add_quad(&mut self, a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>, d: Vector3<f32>) {
        let new_vertices = [
            MeshVertex::from_pos(a),
            MeshVertex::from_pos(b),
            MeshVertex::from_pos(c),
            MeshVertex::from_pos(d),
        ];

        let base_index = self.vertices.len() as u32;
        let new_indices = [
            0 + base_index,
            1 + base_index,
            2 + base_index,
            2 + base_index,
            3 + base_index,
            0 + base_index,
        ];

        self.vertices.extend(new_vertices);
        self.indices.extend(new_indices);
    }
}
