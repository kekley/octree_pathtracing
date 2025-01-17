extern crate ray_tracing;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Ok;
use dashmap::DashMap;
use glam::{UVec3, Vec3A};
use ray_tracing::Camera;
use ray_tracing::Material;
use ray_tracing::OctreeChunkPos;
use ray_tracing::TileRenderer;
use ray_tracing::{MaterialFlags, Octree, Position, RTWImage, Scene, Texture};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use spider_eye::{World, WorldCoords};
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    blocks()?;
    //face_id_test();
    Ok(())
}

fn blocks() -> Result<(), anyhow::Error> {
    const RESOLUTION: (usize, usize) = (1280, 720);
    let camera = Camera::new(
        Vec3A::new(0.0, 8.0, 0.0),
        Vec3A::new(7.0, 8.0, 7.0),
        RESOLUTION.0 as u32,
        70.0,
        RESOLUTION.0 as f32 / RESOLUTION.1 as f32,
    );

    let mut scene = Scene::new().branch_count(1).camera(camera).spp(1).build();
    let world = World::new("./world");

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
    let mut tree: Octree<u32> = Octree::construct_parallel(7, &f);
    tree.set_leaf(UVec3::new(0, 512, 0), 1);
    let arc = Arc::new(tree);
    //println!("{:?}", tree);
    println!("octree built");
    scene.octree = arc;
    let textures = world
        .global_palette
        .read()
        .unwrap()
        .keys()
        .into_iter()
        .filter_map(|str| {
            //println!("{}", str);
            if str.contains("grass") {
                return Some("grass_block".to_string());
            }
            if str.contains("leaves") {
                return Some("leaves".to_string());
            }
            let new_string = str.strip_prefix("minecraft:").unwrap_or(&str).to_string();
            return Some(new_string);
        })
        .collect::<Vec<String>>();
    let texture_path =
        "./assets/default_resource_pack/assets/minecraft/textures/block/".to_string();
    let materials: Arc<Vec<Material>> = textures
        .iter()
        .filter_map(|texture| {
            let image = RTWImage::load((texture_path.clone() + texture + ".png").as_str());
            let albedo = match image {
                Result::Ok(image) => Texture::Image(image),
                Err(_) => Texture::DEFAULT_TEXTURE,
            };
            let material = Material {
                name: texture.into(),
                index_of_refraction: 1.000293,
                material_flags: MaterialFlags::SOLID | MaterialFlags::OPAQUE,
                specular: 0.0,
                emittance: 0.0,
                roughness: 0.0,
                metalness: 0.0,
                albedo: albedo,
            };
            Some(material)
        })
        .collect::<Vec<Material>>()
        .into();
    scene.materials = materials.clone();
    let mut a: TileRenderer = TileRenderer::new(RESOLUTION, 3, 16, scene);
    let start = Instant::now();
    a.render("render.png");
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
    Ok(())
}
