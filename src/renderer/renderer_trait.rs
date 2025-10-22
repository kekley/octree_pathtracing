use std::sync::Arc;

use eframe::egui::{Context, TextureHandle};

use crate::{scene::Scene, settings::RendererBackendSetting};

use super::{
    camera::Camera,
    dummy_renderer::DummyRenderer,
    tile_renderer::{RendererMode, RendererStatus},
};

pub trait ColorScalar {}

impl ColorScalar for f32 {}

impl ColorScalar for u8 {}

pub trait RenderingBackend {
    fn get_camera(&self) -> &Camera;
    fn set_camera(&mut self, camera: Camera);
    fn which_backend(&self) -> RendererBackendSetting;
    fn set_resolution(&mut self, resolution: (u32, u32));
    fn get_status(&self) -> RendererStatus;
    fn get_resolution(&self) -> (u32, u32);
    fn set_mode(&mut self, mode: RendererMode);
    fn get_mode(&self) -> RendererMode;
    fn update_scene(&mut self, ctx: &Context);
    fn set_scene(&mut self, scene: &Arc<parking_lot::RwLock<Scene>>);
    fn render_frame(
        &self,
        egui_frame: &eframe::Frame,
        texture: TextureHandle,
    ) -> Result<Box<dyn FrameInFlight>, TextureHandle>;
}

pub enum FrameInFlightPoll {
    Ready(TextureHandle),
    NotReady(Box<dyn FrameInFlight>),
    Cancelled,
}
pub trait FrameInFlight {
    fn poll(self: Box<Self>) -> FrameInFlightPoll;

    fn wait_for(self: Box<Self>) -> Result<TextureHandle, TextureHandle>;
}

impl Default for Box<dyn RenderingBackend> {
    fn default() -> Self {
        Box::new(DummyRenderer {})
    }
}
