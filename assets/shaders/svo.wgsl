fn read_packed(data :array<u32,8>,start_bit_position:u32)->u32{
    var data_ = data;
    let BITS_PER_CHUNK:u32 = 30u;
    let MASK_30_BITS: u32 = u32((1u<< BITS_PER_CHUNK) - 1);
    if start_bit_position>226{
        return 0u;
    }
    let word_idx:u32 = start_bit_position/32;

    let bit_offset:u32 = start_bit_position%32;

    let u0:u64 = u64(data_[word_idx]);
    var u1:u64 = u64(0);
    if(word_idx+1)<8{
        u1= u64(data_[word_idx+1]);
    }

    let combined:u64 = (u0<<32) | u1;

    let right_shift_amount:u32 = 64-(bit_offset+BITS_PER_CHUNK);

    let extracted:u64 = combined>>right_shift_amount;
    let casted :u32 = u32(extracted);

    return (casted&MASK_30_BITS);
}


struct Octant {
    data: array<u32,8>
}
 
struct TraversalContext {
    octree_scale: f32,
    root: u32,
    scale: u32,
    octant_stack: array<u32,24>,
    time_stack: array<f32,24>,
    padding:u32,
}

@group(0) @binding(0)
var<storage,read> octree: array<Octant>;
@group(0) @binding(1)
var<storage,read> context : TraversalContext;



@group(0) @binding(2)
var output: texture_storage_2d<rgba8unorm,write>;


@group(1) @binding(0)
var texture_array: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var sampler_array: binding_array<sampler>;
@group(1) @binding(2)
var<storage,read> quads: array<Quad>;
@group(1) @binding(3)
var<storage,read> materials: array<Material>;

const u32_max: u32 = 0xFFFFFFFF;
const OCTREE_MAX_SCALE:u32 =23;
const OCTREE_MAX_STEPS:u32 = 1000;
const OCTREE_EPSILON:f32 = 1.1920929e-7;

const camera = CameraParams(vec3(100.0,130.0,20.0),vec3(100.0,12,100.0),vec3(0.0,1.0,0.0),70.0,16.0/9.0);

@compute @workgroup_size(8,8,1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let octree_length = arrayLength(&octree);
    if global_id.x >= 1280 || global_id.y >= 720 {
        return;
    }
    
    let ray = create_ray_optimized(camera,global_id.xy);
    intersect_octree(global_id,ray.origin,ray.direction,1024.0,context);
}


fn max_vec3(v: vec3<f32> )->f32{
    return max(v.x,max(v.y,v.z));
}

fn min_vec3(v: vec3<f32> )->f32{
    return min(v.x,min(v.y,v.z));
}


//creates a bitmask using the three least significant bits of a u32 from a vec3<bool>
fn vec_to_bitmask(v:vec3<bool>)->u32{
    let value:u32 =    select(0u, 1u << 0u, v.x) |
                       select(0u, 1u << 1u, v.y) |
                       select(0u, 1u << 2u, v.z);
    return value;
}

//creates a vec3 of bools based on the three least significant bits of "bits"
fn bitmask_to_vec(bits:u32)->vec3<bool>{
    
    let condition_x: bool = (bits & 1u) != 0u; // Bit 0 for pos.x
    let condition_y: bool = (bits & 2u) != 0u; // Bit 1 for pos.y
    let condition_z: bool = (bits & 4u) != 0u; // Bit 2 for pos.z
    return vec3<bool>(condition_x, condition_y,condition_z);
}


fn intersect_octree(global_id:vec3<u32>,ray_origin: vec3<f32>, ray_direction: vec3<f32>, max_dst: f32, context: TraversalContext) {
    let octree_scale: f32 = context.octree_scale;
    var root: u32 = context.root;
    var scale: u32 = context.scale;
    var octant_stack: array<u32,24> = context.octant_stack;
    var time_stack: array<f32,24> = context.time_stack;
    var ro: vec3<f32> = ray_origin * octree_scale;
    ro += 1.0;
    var current_ptr:u32 = root;
    var rd: vec3<f32> = ray_direction;
    var max_dst_scaled: f32 = max_dst * f32(octree_scale);
    var scale_exp2: f32 = exp2(f32(i32(scale) - i32(OCTREE_MAX_SCALE)));

    var parent_octant_idx: u32 = root;

    var sign_mask: u32 = 1u << 31u;

    let epsilon_bits_without_sign: u32 = bitcast<u32>(OCTREE_EPSILON) & (~sign_mask);


    let rd_abs:vec3<f32> = abs(rd);

    let dir_lt_epsilon: vec3<bool> = rd_abs < vec3(OCTREE_EPSILON);

    let signed_epsilon: vec3<f32> =  bitcast<vec3<f32>>(vec3(epsilon_bits_without_sign) | (bitcast<vec3<u32>>(rd) & vec3(sign_mask)));

    rd = select(rd,signed_epsilon,dir_lt_epsilon); 

    let t_coef:vec3<f32> = 1.0/-abs(rd);

    var t_bias:vec3<f32> = t_coef*ro;

    
    let dir_gt_0: vec3<bool> = rd > vec3(0.0);

    let mirror_mask: u32 = vec_to_bitmask(dir_gt_0);

    let updated_t_bias_values: vec3<f32> = 3.0 * t_coef - t_bias;

    t_bias = select(t_bias, updated_t_bias_values, dir_gt_0);
    
    
    var t_min:f32 = max(max_vec3((2.0 *t_coef-t_bias)),0.0);

    var t_max:f32 = min_vec3(t_coef-t_bias);

    var h :f32 = t_max;

    var idx:u32 = 0;

    var pos : vec3<f32> = vec3(1.0);


    let upper:vec3<f32> = 1.5*t_coef - t_bias;

    let lt_upper : vec3<bool> =  vec3(t_min) < upper;

    idx ^= vec_to_bitmask(lt_upper);
    pos = select(pos,vec3(1.5),lt_upper);
 

    for(var i:u32=0;i<OCTREE_MAX_STEPS;i++){
        if max_dst>=0.0 && t_min>max_dst{
            //miss
            textureStore(output,global_id.xy,vec4(0.0,0.0,0.0,1.0));
            return;
        }


        let t_corner :vec3<f32> = pos*t_coef-t_bias;

        let tc_max:f32 = min_vec3(t_corner);

        let unmirrored_idx:u32 = idx^mirror_mask;
        var octant:Octant = octree[parent_octant_idx];
        
/*         
          switch umirrored_idx{
            case 0u:{
                output[global_id.x+global_id.y*1280] = vec4(0,0,0,1);//red
            }
            case 1u:{

                output[global_id.x+global_id.y*1280] = vec4(0,0,1,1);//blue
            }
            case 2u:{
                output[global_id.x+global_id.y*1280] = vec4(0,1,0,1);//green
            }
            case 3u:{
                output[global_id.x+global_id.y*1280] = vec4(0,1,1,1);//cyan
            }
            case 4u:{
                output[global_id.x+global_id.y*1280] = vec4(1,0,0,1);//red
            }
            case 5u:{
                output[global_id.x+global_id.y*1280] = vec4(1,0,1,1);//magenta
            }
            case 6u:{
                output[global_id.x+global_id.y*1280] = vec4(1,1,0,1);//yellow
            }
            case 7u:{
                output[global_id.x+global_id.y*1280] = vec4(1,1,1,1);//white
            }
            default {

            }
        }  */
 
        var header: u32 = (octant.data[0] >> 16u) & 0xFFFFu;


        let is_child:bool = (header & (1u<<unmirrored_idx))!=0;


        let is_leaf:bool =(header & (1u<<(8+unmirrored_idx)))!=0;


        if is_child && t_min<=t_max{

            if is_leaf&&t_min >=0.0{
                //hit
                let leaf_value:u32 = read_packed(octant.data,16u+unmirrored_idx*30);

                let unmirrored_components:vec3<f32> = 3.0-scale_exp2-pos;
                let unmirror_bools :vec3<bool> = bitmask_to_vec(mirror_mask);
                let unmirrored_pos = select(pos,unmirrored_components,unmirror_bools);


                let t_corner:vec3<f32> = (pos+scale_exp2)*t_coef-t_bias;

                let tc_min = max_vec3(t_corner);


                
                let t_corner_eq_tc_min:vec3<bool> = t_corner==vec3(tc_min);

                let rd_lt_0 : vec3<bool> = rd<vec3(0.0);
                let cond0_active: bool = t_corner_eq_tc_min[0];
                let cond1_active: bool = t_corner_eq_tc_min[1] && !cond0_active;
                let cond2_active: bool = !(cond0_active || cond1_active);

                let sign_rd_0: u32 = bitcast<u32>(rd[0]) >> 31u;
                let sign_rd_1: u32 = bitcast<u32>(rd[1]) >> 31u;
                let sign_rd_2: u32 = bitcast<u32>(rd[2]) >> 31u;

                let face_id_case0: u32 = (1u << 0u) | sign_rd_0;
                let face_id_case1: u32 = (1u << 1u) | sign_rd_1;
                let face_id_case2: u32 = (1u << 2u) | sign_rd_2;

                var face_id: u32 = face_id_case2; 
                face_id = select(face_id, face_id_case1, cond1_active);
                face_id = select(face_id, face_id_case0, cond0_active);

                let uv_raw_case0 = vec2<f32>(
                    (ro[2] + rd[2] * t_corner[0]) - unmirrored_pos[2],
                    (ro[1] + rd[1] * t_corner[0]) - unmirrored_pos[1]
                );
                let uv_raw_case1 = vec2<f32>(
                    (ro[0] + rd[0] * t_corner[1]) - unmirrored_pos[0],
                    (ro[2] + rd[2] * t_corner[1]) - unmirrored_pos[2]
                );
                let uv_raw_case2 = vec2<f32>(
                    (ro[0] + rd[0] * t_corner[2]) - unmirrored_pos[0],
                    (ro[1] + rd[1] * t_corner[2]) - unmirrored_pos[1]
                );

                var uv_selected_raw = uv_raw_case2;
                uv_selected_raw = select(uv_selected_raw, uv_raw_case1, cond1_active);
                uv_selected_raw = select(uv_selected_raw, uv_raw_case0, cond0_active);

                var uv: vec2<f32> = uv_selected_raw / scale_exp2; // Renamed from uv_simd

                let flip_ux_cond: bool = (cond0_active && rd_lt_0[0]) || (cond2_active && rd_lt_0[2]);
                uv.x = select(uv.x, 1.0 - uv.x, flip_ux_cond);

                let flip_uy_cond: bool = cond1_active && rd_lt_0[1];
                uv.y = select(uv.y, 1.0 - uv.y, flip_uy_cond); 

                //hit
                textureStore(output,global_id.xy,vec4(1.0,0.0,1.0,1.0));
                return;
                // if quad_len >0 

            }else{
                //we missed, either because the ray didn't hit anything in front of it or we are not at leaf depth
                let half_scale:f32 = scale_exp2*0.5;

                let t_center:vec3<f32> = half_scale*t_coef+t_corner;

                let tv_max = min(t_max,tc_max);

                if t_min<=tv_max && is_child{
                    //we must descend further into the octree
                    if tc_max<h{
                        octant_stack[scale] = parent_octant_idx;
                        time_stack[scale] = t_max;
                    }

                    h = tc_max;
                    //get the new octant value
                    parent_octant_idx = read_packed(octant.data,16+unmirrored_idx*30);

                    scale-=1;
                    scale_exp2 = half_scale;

                    idx=0;

                    let t_center_gt_t_min:vec3<bool> = t_center > vec3(t_min);
                    let next_pos_components:vec3<f32> = pos+scale_exp2;
                    idx^= vec_to_bitmask(t_center_gt_t_min);
                    pos = select(pos,next_pos_components,t_center_gt_t_min);
                    t_max = tv_max;
                    continue;
                }
            }
        }
        //advance step

        //calculate how to step child index
        let t_corner_le_tc_max :vec3<bool> = t_corner <= vec3(tc_max);
        let step_mask:u32 = vec_to_bitmask(t_corner_le_tc_max);
        let next_pos_components = pos - scale_exp2;
        pos = select(pos,next_pos_components,t_corner_le_tc_max);

        t_min = tc_max;
        idx^=step_mask;

        if (idx&step_mask)!=0{
            //pop step

            let pos_plus_scale: vec3<f32> = pos + scale_exp2; // scale_exp2 is promoted to vec3
            let component_xor_values: vec3<u32> = bitcast<vec3<u32>>(pos) ^ bitcast<vec3<u32>>(pos_plus_scale);

            let conditions: vec3<bool> = bitmask_to_vec(step_mask);
            let xor_contributions: vec3<u32> = select(vec3<u32>(0u), component_xor_values, conditions);

            let differing_bits: u32 = xor_contributions.x | xor_contributions.y | xor_contributions.z;

            scale = firstLeadingBit(differing_bits);

            scale_exp2 = exp2(f32(i32(scale)-i32(OCTREE_MAX_SCALE)));

            if scale>=OCTREE_MAX_SCALE{
                textureStore(output,global_id.xy,vec4(0.0,1.0,0.0,1.0));
                return; //miss
            }

            parent_octant_idx = octant_stack[scale];
            t_max = time_stack[scale];

            let shifted_pos = bitcast<u32>(pos) >> vec3(scale);
            pos = bitcast<f32>(shifted_pos<<vec3(scale));

            idx = (bitcast<u32>(shifted_pos.x)&1) | (bitcast<u32>(shifted_pos.y)&1) <<1 | (bitcast<u32>(shifted_pos.z)&1) <<2;
            h=0.0;
        }
    }
    textureStore(output,global_id.xy,vec4(1.0,0.0,0.0,1.0));
    return; //miss
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

struct Quad{
    origin: vec4<f32>,
    u:  vec4<f32>,
    v:  vec4<f32>,
    u_v_range:  vec4<f32>,
}
// Original CameraParams structure (unchanged)
struct CameraParams {
    position: vec3<f32>,     // Camera's position in world space
    look_at: vec3<f32>,      // Point in world space the camera is looking at
    up: vec3<f32>,           // Up vector in world space (e.g., (0.0, 1.0, 0.0))
    fov: f32,                // Vertical field of view in degrees
    aspect_ratio: f32,       // Width / Height
}

// Original Ray structure (unchanged)
struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

// Global constants
const IMAGE_SIZE: vec2<u32> = vec2<u32>(1280, 720); // Screen resolution in pixels
const PI: f32 = 3.14159265359; // Value of Pi

// Improved and optimized ray generation function
fn create_ray_optimized(
    camera: CameraParams,
    // The 'resolution' parameter was in the original function but IMAGE_SIZE was used.
    // If you intend for the resolution to be dynamic per call,
    // replace IMAGE_SIZE below with 'resolution'.
    // For this example, we'll assume IMAGE_SIZE is the target resolution.
    pixel_id: vec2<u32>    // Current pixel coordinates (e.g., from top-left [0,0])
) -> Ray {
    // 1. Calculate Normalized Device Coordinates (NDC) with Pixel Centering
    // Convert pixel_id to f32 and add 0.5 to sample from the center of the pixel.
    // Then normalize to the range [0, 1].
    let uv_normalized = (vec2<f32>(pixel_id) + vec2<f32>(0.5, 0.5)) / vec2<f32>(IMAGE_SIZE);

    // 2. Convert NDC from [0, 1] to Screen Space [-1, 1]
    // NDC (0,0) is top-left, screen space (-1,1) is top-left for many conventions.
    // Y is often inverted because pixel coordinates usually increase downwards,
    // while view/camera space Y often increases upwards.
    var screen_coords = uv_normalized * 2.0 - 1.0;
    screen_coords.x *= camera.aspect_ratio; // Account for aspect ratio
    screen_coords.y *= -1.0;                // Invert Y-axis

    // 3. Calculate Camera Basis Vectors (View, Up, Right)
    // These vectors define the camera's orientation in world space.

    // view_direction: The direction the camera is looking (forward vector).
    // This is a unit vector pointing from the camera's position to the look_at point.
    let view_direction = normalize(camera.look_at - camera.position);

    // view_up_orthogonal: The camera's "up" vector, made orthogonal to the view_direction.
    // This uses the Gram-Schmidt process to ensure the up vector is truly perpendicular
    // to the viewing direction, preventing skew.
    let view_up_orthogonal = normalize(camera.up - dot(camera.up, view_direction) * view_direction);

    // view_right: The camera's "right" vector.
    // Calculated as the cross product of the view direction and the orthogonal up vector.
    // This assumes a right-handed coordinate system (common in graphics).
    let view_right = cross(view_direction, view_up_orthogonal);

    // 4. Calculate 'd', the distance from the camera origin to the image plane,
    // or more accurately, a scaling factor related to the Field of View (FOV).
    // The vertical FOV is given in degrees, so convert to radians.
    // d = 1.0 / tan(vertical_fov_radians / 2.0)
    let fov_radians = camera.fov * (PI / 180.0);
    let d_factor = 1.0 / tan(fov_radians / 2.0);

    // 5. Define the Ray's Origin in World Space
    // The ray originates from the camera's position.
    let ray_origin = camera.position;

    // 6. Calculate the Ray's Direction in World Space
    // The ray direction is a linear combination of the camera's basis vectors,
    // scaled by the screen coordinates and the d_factor.
    // It points from the camera origin, through the virtual pixel on the image plane,
    // out into the scene.
    let ray_direction_world = normalize(
        d_factor * view_direction +       // Component along the view direction (depth)
        screen_coords.x * view_right +    // Component along the right direction (horizontal)
        screen_coords.y * view_up_orthogonal // Component along the up direction (vertical)
    );

    return Ray(ray_origin, ray_direction_world);
}

// --- Further Optimization: Pre-computation (Conceptual) ---
// If CameraParams are fixed for many ray generations (e.g., per frame),
// many calculations in `create_ray_optimized` are redundant.
// You can pre-calculate parts of this on the CPU or in a setup shader pass.

struct PrecomputedCameraData {
    world_origin: vec3<f32>,
    // Store the three basis vectors scaled appropriately, or the raw vectors and factors
    scaled_view_direction: vec3<f32>, // d_factor * view_direction
    scaled_view_right: vec3<f32>,     // aspect_ratio * view_right (or just view_right)
    view_up_orthogonal: vec3<f32>,    // view_up_orthogonal
    inv_image_size: vec2<f32>,        // 1.0 / IMAGE_SIZE
    // aspect_ratio_val: f32, // if not pre-multiplied into scaled_view_right
};

// This function would be called once when camera parameters change,
// not per-pixel. The result (PrecomputedCameraData) would be passed
// as a uniform to the shader that calls create_ray_from_precomputed.
/*
fn setup_precomputed_camera(camera: CameraParams, image_dims: vec2<u32>) -> PrecomputedCameraData {
    let view_direction = normalize(camera.look_at - camera.position);
    let view_up_orthogonal = normalize(camera.up - dot(camera.up, view_direction) * view_direction);
    let view_right = cross(view_direction, view_up_orthogonal);

    let fov_radians = camera.fov * (PI / 180.0);
    let d_factor = 1.0 / tan(fov_radians / 2.0);

    return PrecomputedCameraData(
        camera.position,
        d_factor * view_direction,
        // If aspect_ratio is applied here: camera.aspect_ratio * view_right,
        // Otherwise, just view_right and multiply by aspect_ratio in the per-pixel shader
        view_right, // Assuming aspect_ratio is handled in the per-pixel shader for this example
        view_up_orthogonal,
        vec2<f32>(1.0, 1.0) / vec2<f32>(image_dims)
        // camera.aspect_ratio // if needed separately
    );
}
*/

// Ray generation using precomputed data (would be much faster per pixel)
fn create_ray_from_precomputed(
    precomp_cam: PrecomputedCameraData, // Passed as a uniform
    pixel_id: vec2<u32>
) -> Ray {
    // 1. Calculate Normalized Device Coordinates (NDC) with Pixel Centering
    let uv_normalized = (vec2<f32>(pixel_id) + vec2<f32>(0.5, 0.5)) * precomp_cam.inv_image_size;

    // 2. Convert NDC from [0, 1] to Screen Space [-1, 1]
    var screen_coords = uv_normalized * 2.0 - 1.0;
    // screen_coords.x *= precomp_cam.aspect_ratio_val; // Apply aspect ratio if not pre-multiplied
    screen_coords.y *= -1.0;

    // 3. Calculate Ray Direction using precomputed vectors
    // If aspect_ratio was pre-multiplied into scaled_view_right:
    // let ray_direction_world = normalize(
    //     precomp_cam.scaled_view_direction +
    //     screen_coords.x * precomp_cam.scaled_view_right + // scaled_view_right already has aspect
    //     screen_coords.y * precomp_cam.view_up_orthogonal
    // );
    // If aspect_ratio is applied here (assuming precomp_cam.scaled_view_right is just view_right):
    // (This also assumes precomp_cam.scaled_view_direction is d_factor * view_direction)
    // And assuming precomp_cam.aspect_ratio_val holds camera.aspect_ratio
    let temp_aspect_ratio = precomp_cam.inv_image_size.y / precomp_cam.inv_image_size.x * (f32(IMAGE_SIZE.x) / f32(IMAGE_SIZE.y)); // Example, if aspect was not in CameraParams but derived. Better to use camera.aspect_ratio

    let ray_direction_world = normalize(
        precomp_cam.scaled_view_direction + // This is d_factor * view_direction
        (screen_coords.x * temp_aspect_ratio) * precomp_cam.scaled_view_right + // This is view_right
        screen_coords.y * precomp_cam.view_up_orthogonal
    );


    return Ray(precomp_cam.world_origin, ray_direction_world);
}