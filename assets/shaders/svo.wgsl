struct Octant{
    header_1_0:u32,
    header_3_2:u32,
    header_5_4:u32,
    header_7_6:u32,
    data: array<u32,8>
}

const u32_max: u32 = 0xFFFFFFFF;

@group(0) @binding(0)
var<storage,read> octree: array<Octant>;

@group(0) @binding(1)
var<storage,read_write> output: array<vec4<f32>>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>){
    let octree_length = arrayLength(&octree);
    if global_id.x>=64|| global_id.y>=64{
        return;
    }
    var color: vec4<f32> = vec4<f32>(0.0);
    var r : f32 = f32(octree[global_id.x%octree_length].header_1_0/u32_max);
    var g : f32 = f32(octree[global_id.x%octree_length].header_3_2/u32_max);
    var b : f32 = f32(octree[global_id.x%octree_length].header_5_4/u32_max);
    color.x = r;
    color.y=g;
    color.z=b;
    color.w=1.0;
    output[global_id.x+global_id.y*64] = color;
}