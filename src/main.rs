extern crate ray_tracing;

use std::{sync::Arc, time::Instant};

use anyhow::Ok;
use glam::{UVec3, Vec3A};
use ray_tracing::{Camera, Octree, Scene, TileRenderer};
use spider_eye::{loaded_world::WorldCoords, ResourceLoader};
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    //face_id_test();
    Ok(())
}

fn blocks() -> Result<(), anyhow::Error> {
    const RESOLUTION: (usize, usize) = (500, 500);
    let camera = Camera::look_at(
        Vec3A::new(0.0, 204.0, 0.0),
        Vec3A::new(100.0, 190.0, 100.0),
        Vec3A::Y,
        70.0,
    );
    let minecraft_loader = ResourceLoader::new();

    let mut scene = Scene::new()
        .branch_count(1)
        .camera(camera)
        .spp(2)
        .build(&minecraft_loader);
    let world = minecraft_loader.open_world("./world");

    let f = |position: UVec3| -> Option<u32> {
        let UVec3 { x, y, z } = position;
        //println!("position: {}", position);
        let block = world.get_block(
            WorldCoords {
                x: (x as i64),
                y: (y as i64 - 64),
                z: (z as i64),
            }
            .into(),
        );
        if block? == 0 {
            //println!("air");
            return None;
        } else {
            //println!("not air");
            return block;
        }
    };

    let mut tree: Octree<u32> = Octree::construct_parallel(8, &f);
    let arc = Arc::new(tree);

    //println!("{:?}", tree);
    println!("octree built");
    scene.octree = arc;
    world.global_palette.into_iter().for_each(|block| {
        if scene
            .resources
            .resource_loader
            .rodeo
            .resolve(&block.block_name)
            != "minecraft:air"
        {
            scene.resources.load_resource(block);
        }
    });
    let mut a: TileRenderer = TileRenderer::new(RESOLUTION, 3, 8, scene);
    let start = Instant::now();
    a.render("render.png");
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
    Ok(())
}
