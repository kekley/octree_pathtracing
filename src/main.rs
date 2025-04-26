extern crate ray_tracing;

use std::{num::NonZeroU32, sync::Arc, time::Instant};

use glam::{UVec3, Vec3A};
use ray_tracing::{
    ray_tracing::{camera::Camera, scene::Scene, tile_renderer::TileRenderer},
    voxels::octree::Octree,
    Application,
};
use spider_eye::{loaded_world::WorldCoords, MCResourceLoader};
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
        "jorkin it",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<Application>::default())
        }),
    )
}
#[test]
fn blocks() -> Result<(), anyhow::Error> {
    const RESOLUTION: (usize, usize) = (1000, 1000);
    let camera = Camera::look_at(
        Vec3A::new(5.0, 17.0, 1.0),
        Vec3A::new(5.0, 4.0, 7.0),
        Vec3A::Y,
        89.0,
    );
    let minecraft_loader = MCResourceLoader::new();

    let mut scene = Scene::new()
        .branch_count(1)
        .camera(camera)
        .spp(2)
        .build(&minecraft_loader);
    let world = minecraft_loader.open_world("./world");
    let air = minecraft_loader.rodeo.get_or_intern("minecraft:air");
    let binding = world
        .get_block(&WorldCoords { x: 0, y: -64, z: 0 })
        .unwrap();
    let should_be_stone_brick = binding.resolve(&minecraft_loader.rodeo);
    let binding = world
        .get_block(&WorldCoords { x: 1, y: -61, z: 0 })
        .unwrap();
    let should_be_cobble = binding.resolve(&minecraft_loader.rodeo);
    let binding = world
        .get_block(&WorldCoords { x: 1, y: -62, z: 0 })
        .unwrap();
    let should_be_dirt = binding.resolve(&minecraft_loader.rodeo);

    dbg!(should_be_stone_brick, should_be_cobble, should_be_dirt);

    let f = |position: UVec3| -> Option<u32> {
        let UVec3 { x, y, z } = position;
        //println!("position: {}", position);
        let block = world.get_block(&WorldCoords {
            x: (x as i64),
            y: (y as i64 - 64),
            z: (z as i64),
        });

        if block.as_ref()?.block_name == air {
            return None;
        } else {
            //println!("not air");
            let model_id = scene.model_manager.load_resource(block.as_ref()?);
            Some(model_id)
        }
    };

    let tree: Octree<u32> = Octree::construct_parallel(8, &f);
    let arc = Arc::new(tree);

    //println!("{:?}", tree);
    println!("octree built");
    scene.octree = arc;

    let mut a: TileRenderer = TileRenderer::new(RESOLUTION, 1, scene);
    let start = Instant::now();
    a.render_to_image("render.png");
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
    Ok(())
}
