use std::f32::consts::PI;

use glam::{Vec3A, Vec4};

use crate::{colors::U8Color, textures::texture::Texture};

#[derive(Debug, Clone)]
pub struct SunConfig {
    sampling_strategy: SunSamplingStrategy,
    pub luminosity: f32,
    pub luminosity_pdf: f32,
    pub importance_sample_chance: f32,
    pub importance_sample_radius: f32,
    draw_texture: bool,
    texture_modification: bool,
    apparent_brightness: f32,
    apparent_texture_brightness: Vec3A,
    texture: Texture,
    color: Vec4,
    sw: Vec3A,
    pub radius: f32,
    pub azimuth: f32,
    pub altitude: f32,
    pub su: Vec3A,
    sv: Vec3A,
    radius_cos: f32,
    radius_sin: f32,
    pub emmittance: Vec4,
}

impl Default for SunConfig {
    fn default() -> Self {
        SunConfig::new(
            PI / 2.5,
            PI / 3.0,
            0.03,
            Vec4::splat(1.0),
            Texture::Color(U8Color::new(255, 255, 255, 255)),
            true,
            false,
            Vec3A::splat(1.0),
        )
    }
}

impl SunConfig {
    pub const DEFAULT_AZIMUTH: f32 = PI / 2.5;
    pub const DEFAULT_ALTITUDE: f32 = PI / 3.0;
    pub const DEFAULT_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.1;
    pub const MAX_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.9;
    pub const MIN_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.001;
    pub const MAX_IMPORTANCE_SAMPLE_RADIUS: f32 = 5.0;
    pub const DEFAULT_IMPORTANCE_SAMPLE_RADIUS: f32 = 1.2;
    pub const MIN_IMPORTANCE_SAMPLE_RADIUS: f32 = 0.1;
    const AMBIENT: f32 = 0.3;
    const INTENSITY: f32 = 1.25;
    const GAMMA: f32 = 2.2;
    pub fn new(
        azimuth: f32,
        altitude: f32,
        radius: f32,
        color: Vec4,
        texture: Texture,
        draw_texture: bool,
        texture_modification: bool,
        apparent_color: Vec3A,
    ) -> Self {
        let radius_cos = radius.cos();
        let radius_sin = radius.sin();

        let theta = azimuth;
        let phi = altitude;

        let r = phi.cos().abs();

        let sw = Vec3A::new(theta.cos() * r, phi.sin(), theta.sin() * r);

        let mut su = if sw.x.abs() > 0.1 {
            Vec3A::new(0.0, 1.0, 0.0)
        } else {
            Vec3A::new(1.0, 0.0, 0.0)
        };

        let mut sv = sw.cross(su);
        sv = sv.normalize();
        su = sv.cross(sw);

        let mut emittance = color;
        emittance *= SunConfig::INTENSITY.powf(SunConfig::GAMMA);
        let apparent_brightness = SunConfig::INTENSITY;
        let mut apparent_texture_brightness = if texture_modification {
            apparent_color
        } else {
            Vec3A::splat(1.0)
        };

        apparent_texture_brightness *= apparent_brightness.powf(SunConfig::GAMMA);

        SunConfig {
            draw_texture,
            texture,
            color,
            sw,
            radius,
            azimuth,
            altitude,
            su,
            sv,
            emmittance: emittance,
            radius_cos,
            radius_sin,
            luminosity: 100.0,
            luminosity_pdf: 1.0 / 100.0,
            importance_sample_chance: SunConfig::DEFAULT_IMPORTANCE_SAMPLE_CHANCE,
            importance_sample_radius: SunConfig::DEFAULT_IMPORTANCE_SAMPLE_RADIUS,
            texture_modification,
            apparent_texture_brightness,
            apparent_brightness,
            sampling_strategy: SunSamplingStrategy::FAST,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SunSamplingStrategy {
    name: &'static str,
    description: &'static str,
    pub sun_sampling: bool,
    pub diffuse_sun: bool,
    pub strict_direct_light: bool,
    pub sun_luminosity: bool,
    pub importance_sampling: bool,
}

impl Default for SunSamplingStrategy {
    fn default() -> Self {
        Self::IMPORTANCE
    }
}

impl SunSamplingStrategy {
    pub const OFF: SunSamplingStrategy = SunSamplingStrategy {
        name: "Off",
        description: "Sun is not sampled with next event estimation.",
        sun_sampling: false,
        diffuse_sun: true,
        strict_direct_light: false,
        sun_luminosity: true,
        importance_sampling: false,
    };

    pub const NON_LUMINOUS: SunSamplingStrategy = SunSamplingStrategy {
        name: "Non-Luminous",
        description: "Sun is drawn on the skybox but it does not contribute to the lighting of the scene.",
        sun_sampling: false,
        diffuse_sun: false,
        strict_direct_light: false,
        sun_luminosity: false,
        importance_sampling: false,
    };

    pub const FAST: SunSamplingStrategy = SunSamplingStrategy {
        name: "Fast",
        description: "Fast sun sampling algorithm. Lower noise but does not correctly model some visual effects.",
        sun_sampling: true,
        diffuse_sun: false,
        strict_direct_light: false,
        sun_luminosity: false,
        importance_sampling: false,
    };

    pub const IMPORTANCE: SunSamplingStrategy = SunSamplingStrategy {
        name: "Importance",
        description: "Sun is sampled on a certain percentage of diffuse reflections. Correctly models visual effects while reducing noise for direct and diffuse illumination.",
        sun_sampling: false,
        diffuse_sun: true,
        strict_direct_light: false,
        sun_luminosity: true,
        importance_sampling: true,
    };

    pub const HIGH_QUALITY: SunSamplingStrategy = SunSamplingStrategy {
        name: "High Quality",
        description: "High quality sun sampling. More noise but correctly models visual effects such as caustics.",
        sun_sampling: true,
        diffuse_sun: true,
        strict_direct_light: true,
        sun_luminosity: true,
        importance_sampling: false,
    };
}
