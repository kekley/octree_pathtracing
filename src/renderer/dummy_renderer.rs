use eframe::egui::TextureHandle;

use super::{camera::Camera, renderer_trait::RenderingBackend, tile_renderer::RendererMode};
static mut DUMMY_CAMERA: Camera = Camera::DEFAULT_CAMERA;

#[derive(Default, Clone, Copy)]
pub struct DummyRenderer {}

impl RenderingBackend for DummyRenderer {
    fn render_frame(
        &self,
        egui_context: &eframe::Frame,
        texture: TextureHandle,
    ) -> Result<Box<dyn super::renderer_trait::FrameInFlight>, TextureHandle> {
        Err(texture)
    }

    fn update_scene(&mut self, ctx: &eframe::egui::Context) {}

    fn set_scene(
        &mut self,
        scene: &std::sync::Arc<parking_lot::RwLock<crate::scene::scene::Scene>>,
    ) {
    }

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

    fn which_backend(&self) -> crate::settings::RendererBackendSetting {
        crate::settings::RendererBackendSetting::Dummy
    }

    fn get_camera(&self) -> &super::camera::Camera {
        &Camera::DEFAULT_CAMERA
    }

    fn set_camera(&mut self, camera: super::camera::Camera) {}
}
