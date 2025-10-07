struct GPUQuad {
    origin: vec3<f32>,
    material_id:u32,
    u: vec3<f32>,
    tex_u_range:u32,
    v: vec3<f32>,
    tex_v_range:u32,
    normal: vec3<f32>,
    plane_d: f32
};

struct GPUMaterial{
    ior: f32,
    specular: f32,
    emittance: f32,
    roughness: f32,
    metalness: f32,
    texture_index: u32,
    tint_index: u32,
    flags: u32,
}

struct Octant {
    data: array<u32,12>
}
 
struct TraversalContext {
    octree_scale: f32,
    root: u32,
    scale: u32,
    padding:u32,
}
