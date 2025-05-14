struct Quad{
    origin:vec4<f32>,
    padding:f32,
    u:vec4<f32>,
    v:vec4<f32>,
    u_v_range:vec4<f32>,
}

struct Material{
    ior: f32,
    specular: f32,
    emittance: f32,
    roughness: f32,
    metalness: f32,
    texture_index: u32,
    padding: u64,
}
struct CameraParams {
    position: vec3<f32>,     // Camera's position in world space
    look_at: vec3<f32>,      // Point in world space the camera is looking at
    up: vec3<f32>,           // Up vector in world space (e.g., (0.0, 1.0, 0.0))
    fov: f32,                // Vertical field of view in degrees
    aspect_ratio: f32,       // Width / Height
}

// Ray structure for WGSL
struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

fn convert_vec2u32_to_vec2f(v_u32: vec2<u32>) -> vec2<f32> {
    return vec2<f32>(f32(v_u32.x), f32(v_u32.y));
}

// Updated create_ray function for WGSL
// Generates a ray in world space for a given pixel,
// using the camera's position, look_at point, and up vector.
fn create_ray(
    camera: CameraParams,
    resolution: vec2<u32>, // viewport resolution in pixels
    pixel_id: vec2<u32>    // current pixel coordinates (e.g., from top-left)
) -> Ray {
    // Convert u32 vectors to f32 vectors for calculations
    let pixel_coord_f: vec2<f32> = convert_vec2u32_to_vec2f(pixel_id);
    let resolution_f: vec2<f32> = convert_vec2u32_to_vec2f(resolution);

    // 1. Normalized pixel coordinates (0 to 1 range)
    // Add 0.5 to sample pixel centers, common in ray tracing.
    let uv: vec2<f32> = vec2<f32>(
        (pixel_coord_f.x + 0.5) / resolution_f.x,
        (pixel_coord_f.y + 0.5) / resolution_f.y
    );

    // 2. Normalized Device Coordinates (NDC)
    // Range from -1 to 1. Y is flipped (1 at top to -1 at bottom for many APIs).
    // (1.0 - uv.y) means uv.y=0 (top in normalized image coords) -> ndc.y=1 (top in NDC)
    let ndc: vec2<f32> = vec2<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0 // Y flipped: uv.y=0 (top) -> ndc.y=1 (top)
    );

    // 3. Calculate view space coordinates on the image plane
    // camera.fov is assumed to be the vertical field of view.
    // tan(fov/2) gives half the height of the image plane at distance 1.
    let fov_tan_half: f32 = tan(radians(camera.fov) * 0.5);

    // The components of the direction in camera's local coordinate system
    // (X right, Y up, looking along -Z)
    // Aspect ratio is applied to the X component to correct for screen shape.
    let local_ray_x: f32 = ndc.x * camera.aspect_ratio * fov_tan_half;
    let local_ray_y: f32 = ndc.y * fov_tan_half;
    let local_ray_z: f32 = -1.0; // Ray points along the camera's local -Z axis

    // 4. Determine camera's coordinate system in world space
    // This defines the camera's orientation.

    // Forward vector: direction from camera position to look_at point
    // WGSL subtraction of vec3 is component-wise.
    let cam_forward_dir: vec3<f32> = normalize(camera.look_at - camera.position);

    // Right vector: cross product of forward and world up vector.
    // Ensure camera.up is not collinear with cam_forward_dir for robust behavior.
    // (Robust handling for collinear cases is omitted for brevity here but important in practice)
    let cam_right_dir: vec3<f32> = normalize(cross(cam_forward_dir, camera.up));
    
    // Actual Up vector for the camera: re-calculate by crossing right and forward.
    // This ensures the up vector is orthogonal to both right and forward vectors.
    let cam_up_actual_dir: vec3<f32> = normalize(cross(cam_right_dir, cam_forward_dir));

    // 5. Transform the local ray direction to world space
    // The local ray (local_ray_x, local_ray_y, local_ray_z) is in camera space.
    // We transform it using the camera's basis vectors (cam_right_dir, cam_up_actual_dir, cam_forward_dir).
    // local_ray_z is -1.0, meaning 1.0 unit along the cam_forward_dir direction (camera's -Z local is world forward).
    // WGSL multiplication of vec3 by scalar is component-wise.
    let world_direction_unnormalized: vec3<f32> =
        cam_right_dir * local_ray_x +       // X component along camera's right
        cam_up_actual_dir * local_ray_y +   // Y component along camera's up
        cam_forward_dir * (-local_ray_z);   // Z component along camera's forward
                                            // (-local_ray_z is +1.0, so it's 1.0 * cam_forward_dir)

    let world_direction: vec3<f32> = normalize(world_direction_unnormalized);

    // 6. The ray's origin is the camera's position
    let ray_origin: vec3<f32> = camera.position;

    return Ray(ray_origin, world_direction); // WGSL struct constructor syntax
}