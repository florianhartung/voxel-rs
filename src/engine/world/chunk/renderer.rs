use crate::engine::rendering::{RenderCtx, Renderer};
use crate::engine::world::mesh::renderer::MeshRenderer;
use wgpu::{BindGroup, RenderPass};

pub struct ChunkRenderer {
    pub mesh_renderer: MeshRenderer,
}

impl Renderer for ChunkRenderer {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        self.mesh_renderer.render(render_pass, camera_bind_group);
    }
}
