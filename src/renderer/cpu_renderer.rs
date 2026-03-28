use eframe::egui::{ColorImage, TextureFilter, TextureOptions, TextureWrapMode};

use crate::{
    colors::{F32Color, U8Color},
    geometry::aabb::AABB,
    octree::{world::WorldOctree, world_svo_intersect::intersect_world_svo_simple},
    path_tracing::Intersect as _,
    renderer::{camera::Camera, renderer_trait::Renderer},
};

impl Renderer for CpuRenderer {
    fn get_camera(&self) -> &Camera {
        &self.camera
    }

    fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    fn set_resolution(&mut self, resolution: (u32, u32)) {}

    fn get_status(&self) -> super::tile_renderer::RendererStatus {
        super::tile_renderer::RendererStatus::Running
    }

    fn get_resolution(&self) -> (u32, u32) {
        (self.resolution[0], self.resolution[1])
    }

    fn set_mode(&mut self, mode: super::tile_renderer::RendererMode) {}

    fn get_mode(&self) -> super::tile_renderer::RendererMode {
        super::tile_renderer::RendererMode::Preview
    }

    fn update_scene(&mut self, ctx: &eframe::egui::Context) {}

    fn update(&mut self, tree: &WorldOctree) {
        self.render_frame(tree);
    }

    fn render_frame_to_texture(&self, mut texture: eframe::egui::TextureHandle) {
        let size = [self.resolution[0] as usize, self.resolution[1] as usize];
        let bytes = U8Color::to_byte_slice(&self.buffer);
        let image = ColorImage::from_rgba_unmultiplied(size, bytes);
        texture.set(
            image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                minification: TextureFilter::Linear,
                wrap_mode: TextureWrapMode::ClampToEdge,
                mipmap_mode: None,
            },
        );
    }

    fn get_backend_type(&self) -> crate::settings::BackendType {
        crate::settings::BackendType::CPU
    }
}

pub struct CpuRenderer {
    resolution: [u32; 2],
    buffer: Vec<U8Color>,
    camera: Camera,
}

#[derive(Default)]
pub struct CpuRendererBuilder {
    resolution: Option<[u32; 2]>,
    camera: Option<Camera>,
}

impl CpuRendererBuilder {
    pub fn with_camera(self, camera: Camera) -> Self {
        Self {
            camera: Some(camera),
            ..self
        }
    }
    pub fn with_resolution(self, resolution: [u32; 2]) -> Self {
        Self {
            resolution: Some(resolution),
            ..self
        }
    }
    pub fn build(self) -> CpuRenderer {
        let resolution = self.resolution.unwrap_or([500, 500]);
        let buffer = vec![U8Color::BLACK; (resolution[0] * resolution[1]) as usize];

        CpuRenderer {
            buffer,
            camera: self.camera.unwrap_or_default(),
            resolution,
        }
    }
}

impl CpuRenderer {
    pub fn builder() -> CpuRendererBuilder {
        CpuRendererBuilder::default()
    }
    pub fn render_frame(&mut self, octree: &WorldOctree) {
        for y in 1..self.resolution[1] + 1 {
            for x in 1..self.resolution[0] + 1 {
                let (x_normalized, y_normalized) = normalize_screen_coords(x, y, self.resolution);
                let ray = self.camera.get_ray(x_normalized, y_normalized);
                if let Some(intersection) = intersect_world_svo_simple(octree, &ray) {
                    print!("x: {x}, y: {y}");
                    let stride = self.resolution[0] as usize;
                    let pos = intersection.position;
                    print!("pos: {pos}");

                    let floored_pos = pos.floor();
                    print!("floored_pos: {floored_pos}");

                    let aabb = AABB::new(floored_pos, floored_pos + 1.0);
                    if let Some(intersection) = aabb.intersect(&ray) {
                        let normal = intersection.normal;
                        print!("normal: {normal}");

                        let color = F32Color::new(normal.x, normal.y, normal.z, 1.0);
                        self.buffer[(y as usize * stride) + x as usize] = color.into();
                    }
                    println!();
                }
            }
        }
    }
}

fn normalize_screen_coords(x: u32, y: u32, resolution: [u32; 2]) -> (f32, f32) {
    let x_normalized = ((x * 2) as f32 / resolution[0] as f32) - 1.0;
    let y_normalized = ((y * 2) as f32 / resolution[1] as f32) - 1.0;
    assert!((-1.0..=1.0).contains(&x_normalized) && (-1.0..=1.0).contains(&y_normalized));
    //    println!("x: {x}->{x_normalized} , y: {y}->{y_normalized}");

    (x_normalized, y_normalized)
}

#[test]
fn coord_normalize() {
    let mut c = CpuRenderer::builder().build();
    let tree = WorldOctree::default();
    c.render_frame(&tree);
}
