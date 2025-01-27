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
use spider_eye::block_states::BlockState;
use spider_eye::loaded_world::World;
use spider_eye::loaded_world::WorldCoords;
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    blocks()?;
    //face_id_test();
    Ok(())
}

fn blocks() -> Result<(), anyhow::Error> {
    const RESOLUTION: (usize, usize) = (1000, 1000);
    let camera = Camera::look_at(
        Vec3A::new(0.0, 204.0, 0.0),
        Vec3A::new(100.0, 190.0, 100.0),
        Vec3A::Y,
        70.0,
    );

    let mut scene = Scene::new().branch_count(10).camera(camera).spp(20).build();
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
    let mut tree: Octree<u32> = Octree::construct_parallel(9, &f);
    let arc = Arc::new(tree);
    //println!("{:?}", tree);
    println!("octree built");
    scene.octree = arc;
    let textures = world
        .global_palette
        .into_iter()
        .filter_map(|state| {
            //println!("{}", str);
            if state.block_name().contains("grass") {
                return Some("grass_block".to_string());
            }
            if state.block_name().contains("leaves") {
                return Some("leaves".to_string());
            }
            if state.block_name().contains("water") {
                return Some("water".to_string());
            }
            let new_string = state
                .block_name()
                .strip_prefix("minecraft:")
                .unwrap_or(&state.block_name())
                .to_string();
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
    let mut a: TileRenderer = TileRenderer::new(RESOLUTION, 3, 1, scene);
    let start = Instant::now();
    a.render("render.png");
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
    Ok(())
}

fn face_id_test() {
    let mut tree: Octree<u32> = Octree::new();

    let camera = Camera::look_at(Vec3A::new(0.0, 0.0, 0.0), Vec3A::splat(2.0), Vec3A::Y, 70.0);

    tree.set_leaf(UVec3::splat(2), 0);

    let arc_tree = Arc::new(tree);
    let materials = vec![Material::default()];
    let arc_mat = Arc::new(materials);
    let mut scene = Scene::new().branch_count(1).spp(1).camera(camera).build();
    scene.octree = arc_tree;
    scene.materials = arc_mat;

    let mut renderer = TileRenderer::new((1000, 1000), 3, 1, scene);

    renderer.render("./test.png");
}
