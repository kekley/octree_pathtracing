use std::{sync::Arc, thread};

use rand::{rngs::StdRng, SeedableRng};

use crate::{path_tracer::*, Camera, Ray, Scene};
struct Tile {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

pub struct TileRenderer {
    pub thread_count: usize,
    pub resolution: (u32, u32),
    scene: Box<Scene>,
}

impl TileRenderer {
    pub fn new(render_resolution: (u32, u32), threads: usize, scene: Box<Scene>) -> Self {
        Self {
            thread_count: threads,
            resolution: render_resolution,
            scene,
        }
    }

    pub fn render(&self) {}

    fn render_tile(tile: Tile, scene: &Scene) {
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let mut ray = scene.camera.get_ray(&mut rng, x, y);
                path_trace(scene, &mut ray, true);
            }
        }
    }
}
