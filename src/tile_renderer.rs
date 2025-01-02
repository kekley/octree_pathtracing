use std::{sync::Arc, thread};

use rand::{rngs::StdRng, SeedableRng};

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
    scene: Arc<Scene>,
}

impl TileRenderer {
    pub fn new(render_resolution: (u32, u32), threads: usize, scene: Scene) -> Self {
        Self {
            thread_count: threads,
            resolution: render_resolution,
            scene: Arc::new(scene),
        }
    }

    pub fn render(&self) {
        let mut threads = vec![];
        let tile_width = self.resolution.0 / self.thread_count as u32;
        let tile_height = self.resolution.1 / self.thread_count as u32;

        for y in 0..self.thread_count {
            for x in 0..self.thread_count {
                let tile = Tile {
                    x0: x as u32 * tile_width,
                    y0: y as u32 * tile_height,
                    x1: (x as u32 + 1) * tile_width,
                    y1: (y as u32 + 1) * tile_height,
                };
                let scene = self.scene.clone();
                threads.push(thread::spawn(move || {
                    TileRenderer::render_tile(tile, &scene);
                }));
            }
        }

        for t in threads {
            t.join().unwrap();
        }
    }

    pub fn render_tile(tile: Tile, scene: &Scene) {
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let color = scene.trace_ray(x, y, &mut rng);
                println!("final pixel color: {:?}", color);
            }
        }
    }
}
