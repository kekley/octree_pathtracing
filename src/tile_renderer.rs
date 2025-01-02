use std::{
    fs::File,
    io::{self, Write},
    sync::Arc,
    thread, vec,
};

use glam::Vec4Swizzles;
use rand::{rngs::StdRng, SeedableRng};

use crate::{path_tracer::*, scene, util, Camera, Ray, Scene};
pub struct Tile {
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

    pub fn single_thread_render(&self) {
        let mut rng = StdRng::from_entropy();
        let mut buffer = Vec::new();
        let header = format!("P3\n{} {}\n255\n", self.resolution.0, self.resolution.1);

        buffer.write_all(header.as_bytes()).unwrap();
        for y in 0..self.resolution.1 {
            for x in 0..self.resolution.0 {
                let (norm_x, norm_y) =
                    TileRenderer::normalize_coordinates(x, y, self.resolution.0, self.resolution.1);
                let color = self.scene.trace_ray(norm_x, norm_y, &mut rng);
                util::write_rgb8_color_as_text_to_stream(&color.xyz(), &mut buffer);
            }
        }
        let mut stream = File::create("tile.ppm").unwrap();
        stream.write_all(&buffer).unwrap();
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
        let mut buffer = Vec::with_capacity(
            (tile.x1 - tile.x0) as usize * (tile.y1 - tile.y0) as usize * 12 + 64,
        );
        let header = format!("P3\n{} {}\n255\n", tile.x1 - tile.x0, tile.y1 - tile.y0);
        buffer.write_all(header.as_bytes()).unwrap();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let (norm_x, norm_y) = TileRenderer::normalize_coordinates(x, y, tile.x1, tile.y1);
                let color = scene.trace_ray(norm_x, norm_y, &mut rng);
                util::write_rgb8_color_as_text_to_stream(&color.xyz(), &mut buffer);
            }
        }
        let mut stream = File::create("tile.ppm").unwrap();
        stream.write_all(&buffer).unwrap();
    }

    fn normalize_coordinates(x: u32, y: u32, width: u32, height: u32) -> (f32, f32) {
        let dim = width.max(height) as f32;
        let xn = ((2 * x + 1) as f32 - width as f32) / dim;
        let yn = ((2 * (height - y) - 1) as f32 - height as f32) / dim;

        (xn, yn)
    }
}
