use std::{future::Future, marker::PhantomData, sync::Arc, thread::JoinHandle};

use eframe::{
    egui::{mutex::Mutex, TextureHandle},
    wgpu::{self, Device, Queue, SubmissionIndex},
};
use parking_lot::{lock_api::MutexGuard, RawMutex, RwLock};

use crate::{gpu_structs::gpu_octree::TraversalContext, scene::scene::Scene};

pub trait ColorScalar {}

impl ColorScalar for f32 {}

impl ColorScalar for u8 {}

pub trait RenderingBackend {
    fn render_frame(
        &self,
        texture: egui_wgpu::Texture,
    ) -> Result<Box<dyn super::renderer_trait::FrameInFlight>, egui_wgpu::Texture>;
}

pub enum FrameInFlightPoll<'a, T: FrameInFlight<'a>> {
    Ready(egui_wgpu::Texture),
    NotReady(&'a T),
    Cancelled,
}
pub trait FrameInFlight<'a> {
    fn poll(self) -> FrameInFlightPoll<'a, Self>
    where
        Self: Sized;

    fn wait_for(self) -> Result<egui_wgpu::Texture, egui_wgpu::Texture>;
}
