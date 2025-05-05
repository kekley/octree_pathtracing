use glam::{Vec2, Vec3A as Vec3, Vec4};

const BACKGROUND_1_COLOR: Vec3 = Vec3::splat(0.9);
const COLOR_3: Vec3 = Vec3::new(0.2, 0.0, 0.6);

const BOX_SIZE_X: f32 = 1.0;

fn wrap(mut x: f32, a: f32, s: f32) -> f32 {
    x -= s;
    return (x - a * (x / a).floor()) + s;
}

fn trans_a(z: &mut Vec2, a: f32, b: f32) {
    let i_r = 1.0 / z.dot(*z);

    *z *= -i_r;
    z.x = -b - z.x;
    z.y = a + z.y;
}

fn jos_kleinian(mut z: Vec2, time: f32) -> f32 {
    let mut lz = z + Vec2::splat(1.0);
    let mut llz = z + Vec2::splat(-1.0);
    let mut flag: f32 = 0.0;
    let klein_r = 1.8462756
        + (1.958591 - 1.8462756) * 0.5
        + 0.5 * (1.958591 - 1.8462756) * (-time * 0.2).sin();
    let klein_i = 0.09627581
        + (0.0112786 - 0.09627581) * 0.5
        + 0.5 * (0.0112786 - 0.09627581) * (-time * 0.2).sin();
    let a = klein_r;
    let b = klein_i;
    let f = b.signum() * 1.;

    for _ in 0..150 {
        z.x = z.x + f * b / a * z.y;
        z.x = wrap(z.x, 2. * BOX_SIZE_X, -BOX_SIZE_X);
        z.x = z.x - f * b / a * z.y;

        //If above the separation line, rotate by 180° about (-b/2, a/2)
        if z.y
            >= a * 0.5
                + f * (2. * a - 1.95) / 4.
                    * (z.x + b * 0.5).signum()
                    * (1.0 - f32::exp(-(7.2 - (1.95 - a) * 15.) * (z.x + b * 0.5).abs()))
        {
            z = Vec2::new(-b, a) - z;
        }

        //Apply transformation a
        trans_a(&mut z, a, b);

        //
        //If the iterated points enters a 2-cycle , bail out.
        if (z - llz).dot(z - llz) < 1e-6 {
            break;
        }
        //if the iterated point gets outside z.y=0 and z.y=a
        if z.y < 0. || z.y > a {
            flag = 1.0;
            break;
        }
        //Store prévious iterates
        llz = lz;
        lz = z;
    }
    return flag;
}

pub fn main_image(x: f32, y: f32, resolution: Vec2, time: f32) -> Vec4 {
    let frag_coord = Vec2::new(x, y);
    let mut uv = frag_coord / resolution;

    uv = (1.99) * uv - Vec2::new(0.42, 0.0);
    uv.x *= resolution.x / resolution.y;

    let hit = jos_kleinian(uv, time);
    let c = (1.0 - hit) * BACKGROUND_1_COLOR + hit * COLOR_3;

    return Vec4::new(c.x, c.y, c.z, 1.0);
}
