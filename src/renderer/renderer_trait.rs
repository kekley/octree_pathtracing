use std::sync::Arc;

use eframe::egui::{Context, TextureHandle};

use crate::{octree::world::WorldOctree, scene::Scene, settings::BackendType};

use super::{
    camera::Camera,
    dummy_renderer::DummyRenderer,
    tile_renderer::{RendererMode, RendererStatus},
};

pub trait ColorScalar {}

impl ColorScalar for f32 {}

impl ColorScalar for u8 {}

pub trait Renderer {
    fn get_camera(&self) -> &Camera;
    fn set_camera(&mut self, camera: Camera);
    fn get_backend_type(&self) -> BackendType;
    fn set_resolution(&mut self, resolution: (u32, u32));
    fn get_status(&self) -> RendererStatus;
    fn get_resolution(&self) -> (u32, u32);
    fn set_mode(&mut self, mode: RendererMode);
    fn get_mode(&self) -> RendererMode;
    fn update_scene(&mut self, ctx: &Context);
    fn update(&mut self, tree: &WorldOctree);
    fn render_frame_to_texture(&self, texture: TextureHandle);
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

impl Default for Box<dyn Renderer> {
    fn default() -> Self {
        Box::new(DummyRenderer {})
    }
}
