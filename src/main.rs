extern crate ray_tracing;

use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{UVec3, Vec3A};
use ray_tracing::{
    ray_tracing::{camera::Camera, scene::Scene, tile_renderer::TileRenderer},
    voxels::octree::Octree,
    Application,
};
use spider_eye::{
    loaded_world::{World, WorldCoords},
    MCResourceLoader,
};
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    //face_id_test();
    ui().unwrap();
    Ok(())
}

fn ui() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "f",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<Application>::default())
        }),
    )
}

#[test]
fn lichen() {
    let loader = MCResourceLoader::new();
    let world = loader.open_world("./biggerworld");
    let lichen = world
        .get_block(&WorldCoords {
            x: 16 + 8,
            y: -11,
            z: 16 + 15,
        })
        .unwrap();

    let resolved = lichen.resolve(&loader);
    dbg!(resolved);
}
