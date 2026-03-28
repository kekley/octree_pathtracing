use eframe::egui::TextureHandle;

use crate::scene::Scene;

use super::{camera::Camera, renderer_trait::Renderer, tile_renderer::RendererMode};
static mut DUMMY_CAMERA: Camera = Camera::DEFAULT_CAMERA;

#[derive(Default, Clone, Copy)]
pub struct DummyRenderer {}

impl Renderer for DummyRenderer {
    fn render_frame_to_texture(&self, texture: TextureHandle) {}

    fn update_scene(&mut self, ctx: &eframe::egui::Context) {}

    fn get_mode(&self) -> super::tile_renderer::RendererMode {
        super::tile_renderer::RendererMode::Preview
    }

    fn get_status(&self) -> super::tile_renderer::RendererStatus {
        super::tile_renderer::RendererStatus::Stopped
    }

    fn get_resolution(&self) -> (u32, u32) {
        (1280, 720)
    }

    fn set_resolution(&mut self, resolution: (u32, u32)) {}

    fn set_mode(&mut self, mode: RendererMode) {}

    fn get_backend_type(&self) -> crate::settings::BackendType {
        crate::settings::BackendType::Dummy
    }

    fn get_camera(&self) -> &super::camera::Camera {
        &Camera::DEFAULT_CAMERA
    }

    fn set_camera(&mut self, camera: super::camera::Camera) {}

    fn update(&mut self, tree: &crate::octree::world::WorldOctree) {}
}
