use crate::engine::rendering::RenderCtx;
use crate::engine::world::mesh::renderer::MeshRenderer;

pub mod renderer;

pub struct MeshVertex {
    pub(crate) position: [f32; 3],
}

impl MeshVertex {
    pub fn from_pos(position: [f32; 3]) -> Self {
        Self { position }
    }
}

pub struct Mesh {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }

    pub fn get_renderer(
        &self,
        render_ctx: &RenderCtx,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> MeshRenderer {
        MeshRenderer::new(
            render_ctx,
            &self.vertices,
            &self.indices,
            camera_bind_group_layout,
        )
    }

    pub fn update_renderer(&self, mesh_renderer: &mut MeshRenderer, render_ctx: &RenderCtx) {
        mesh_renderer.update_data(&self.vertices, &self.indices, render_ctx);
    }
}
