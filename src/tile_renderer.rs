use std::{sync::Arc, thread};


use crate::{path_tracer::*, scene, Camera, Ray, Scene};
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
        let mut rng = Rng::new();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let mut ray = Camera::thread_safe_get_ray(center, pixel_delta_u, pixel_delta_v, pixel00_loc, defocus_angle, disc_u, disc_v, &mut rng, x, y)
                path_trace(&mut ray, true);
            }
        }
    }
}
