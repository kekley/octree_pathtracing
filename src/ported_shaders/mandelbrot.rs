use crate::smoothstep;
use glam::Vec3A as Vec3;
use glam::{Vec2, Vec4};
const AA: usize = 2;
pub fn mandelbrot(x: f32, y: f32, resolution: Vec2, time: f32) -> Vec4 {
    let frag_coord = Vec2::new(x, y);
    let mut col = Vec3::splat(0.0);

    for m in 0..AA {
        for n in 0..AA {
            let p = (2.0 * (frag_coord + Vec2::new(m as f32, n as f32) / AA as f32) - resolution)
                / resolution.y;
            let zoo = 1.0 / (350.0 - 250.0 * (0.25 * time - 0.3).sin());

            let cc = Vec2::new(-0.533516, 0.526141) + p * zoo;

            let mut t2c = Vec2::new(-0.5, 2.0);

            t2c += 0.5 * Vec2::new((0.13 * (time - 10.0)).cos(), (0.13 * (time - 10.0)).sin());

            let mut z = Vec2::splat(0.0);
            let mut dz = Vec2::splat(0.0);

            let mut trap1: f32 = 0.0;
            let mut trap2: f32 = 1e20;
            let mut co2: f32 = 0.0;

            for _ in 0..150 {
                dz = 2.0 * Vec2::new(z.x * dz.x - z.y * dz.y, z.x * dz.y + z.y * dz.x)
                    + Vec2::new(1.0, 0.0);

                z = cc + Vec2::new(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);

                let d1: f32 = (z - Vec2::new(0.0, 1.0)).dot(Vec2::splat(0.707)).abs();
                let ff = 1.0 - smoothstep(0.6, 1.4, d1);
                co2 += ff;
                trap1 += ff * d1;

                trap2 = (trap2).min((z - t2c).dot(z - t2c));

                if z.dot(z) > 1024.0 {
                    break;
                }
            }

            let d: f32 = (z.dot(z) / dz.dot(dz)).sqrt() * z.dot(z).ln();

            let c1 = (2.00 * d / zoo).clamp(0.0, 1.0).powf(0.5);
            let c2 = (1.5 * trap1 / co2).clamp(0.0, 1.0).powf(2.0);
            let c3 = (0.4 * trap2).clamp(0.0, 1.0).powf(0.25);

            let mut arr1 = (3.0 + 4.0 * c2 + Vec3::new(0.0, 0.5, 1.0)).to_array();

            arr1.iter_mut().for_each(|f| {
                *f = f.sin();
            });
            let col1 = 0.5 + 0.5 * Vec3::from_array(arr1);
            let mut arr2 = (4.1 + 2.0 * c3 + Vec3::new(1.0, 0.5, 0.0)).to_array();
            arr2.iter_mut().for_each(|f| {
                *f = f.sin();
            });
            let col2 = 0.5 + 0.5 * Vec3::from_array(arr2);
            let mut arr3 = (c1 * col1 * col2).to_array();
            arr3.iter_mut().for_each(|f| *f = f.sqrt());
            col += 2.0 * Vec3::from_array(arr3);
        }
    }
    col /= (AA * AA) as f32;

    Vec4::new(
        col.x.clamp(0.0, 1.0),
        col.y.clamp(0.0, 1.0),
        col.z.clamp(0.0, 1.0),
        1.0,
    )
}
