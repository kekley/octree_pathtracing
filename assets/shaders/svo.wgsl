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

struct Octant {
    header: array<u32,4>,
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
var<storage,read_write> output: array<vec4<f32>>;
const u32_max: u32 = 0xFFFFFFFF;
const OCTREE_MAX_SCALE:u32 =23;
const OCTREE_MAX_STEPS:u32 = 1000;
const OCTREE_EPSILON:f32 = 1.1920929e-7;

const camera = CameraParams(vec3(4.0,130.0,1.0),vec3(7.0,130,7.0),vec3(0.0,1.0,0.0),70.0,16.0/9.0);

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let octree_length = arrayLength(&octree);
    if global_id.x >= 1280 || global_id.y >= 720 {
        return;
    }
    
    let ray = create_ray(camera,vec2(1280,720),global_id.xy);
    output[global_id.x+global_id.y*1280] = vec4(ray.direction.x,ray.direction.y,ray.direction.z,1.0);
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
    var octant_stack: array<u32,OCTREE_MAX_SCALE+1> = context.octant_stack;
    var time_stack: array<f32,OCTREE_MAX_SCALE+1> = context.time_stack;
    var ro: vec3<f32> = ray_origin * octree_scale;
    ro += 1.0;
    var current_ptr:u32 = root;
    var rd: vec3<f32> = ray_direction;
    var max_dst_scaled: f32 = max_dst * f32(octree_scale);
    var scale_exp2: f32 = exp2(f32(i32(scale) - i32(OCTREE_MAX_SCALE)));

    var parent_octant_idx: u32 = root;

    var sign_mask: u32 = u32(1) << 31;

    let epsilon_bits_without_sign: u32 = bitcast<u32>(OCTREE_EPSILON) & (~sign_mask);

    if abs(rd.x) < OCTREE_EPSILON{
        rd.x = bitcast<f32>(epsilon_bits_without_sign | (bitcast<u32>(rd.x) & sign_mask));
    }
        if abs(rd.y) < OCTREE_EPSILON{
        rd.y = bitcast<f32>(epsilon_bits_without_sign | (bitcast<u32>(rd.y) & sign_mask));
    }
        if abs(rd.z) < OCTREE_EPSILON{
        rd.z = bitcast<f32>(epsilon_bits_without_sign | (bitcast<u32>(rd.z) & sign_mask));
    }
        output[global_id.x+global_id.y*1280] = vec4(rd,1);

/*     let rd_abs:vec3<f32> = abs(rd);

    let dir_lt_epsilon: vec3<bool> = rd_abs < vec3(OCTREE_EPSILON);

    let signed_epsilon: vec3<f32> =  bitcast<vec3<f32>>(vec3(epsilon_bits_without_sign) | (bitcast<vec3<u32>>(rd) & vec3(sign_mask)));

    rd = select(rd,signed_epsilon,dir_lt_epsilon); */

    let t_coef:vec3<f32> = 1.0/-abs(rd);

    var t_bias:vec3<f32> = t_coef*ro;

    var mirror_mask:u32 = 0;
    if rd.x>0{
        mirror_mask ^= 1;
        t_bias.x = 3.0*t_coef.x -t_bias.x;
    }
    if rd.y>0{
        mirror_mask ^= 2;
        t_bias.y = 3.0*t_coef.y -t_bias.y;
    }
    if rd.z>0{
        mirror_mask ^= 4;
        t_bias.z = 3.0*t_coef.z -t_bias.z;
    }

    /* 
    let dir_gt_0: vec3<bool> = rd > vec3(0.0);

    let mirror_mask: u32 = vec_to_bitmask(dir_gt_0);

    let updated_t_bias_values: vec3<f32> = 3.0 * t_coef - t_bias;

    t_bias = select(t_bias, updated_t_bias_values, dir_gt_0);
     */
    
    
    var t_min:f32 = max(max_vec3((2.0 *t_coef-t_bias)),0.0);

    var t_max:f32 = min_vec3(t_coef-t_bias);

    var h :f32 = t_max;

    var idx:u32 = 0;

    var pos : vec3<f32> = vec3(1.0);

    if t_min< (1.5*t_coef.x-t_bias.x){
        idx ^= 1;
        pos.x=1.5;
    }
    
    if t_min< (1.5*t_coef.y-t_bias.y){
        idx ^= 2;
        pos.y=1.5;
    }
    
    if t_min< (1.5*t_coef.z-t_bias.z){
        idx ^= 4;
        pos.z=1.5;
    }

/*     let upper:vec3<f32> = 1.5*t_coef - t_bias;

    let lt_upper : vec3<bool> =  vec3(t_min) < upper;

    idx ^= vec_to_bitmask(lt_upper);
    pos = select(pos,vec3(1.5),lt_upper);
 */

    for(var i:u32=0;i<OCTREE_MAX_STEPS;i++){
        if max_dst>=0.0 && t_min>max_dst{
            //miss
            return;
        }


        let t_corner :vec3<f32> = pos*t_coef-t_bias;

        let tc_max:f32 = min_vec3(t_corner);

        let unmirrored_idx:u32 = idx^mirror_mask;
        let octant:Octant = octree[parent_octant_idx];
          switch unmirrored_idx{
            case 0u:{
                output[global_id.x+global_id.y*1280] = vec4(1,0,0,1);//red
            }
            case 1u:{

                output[global_id.x+global_id.y*1280] = vec4(0,1,0,1);//green
            }
            case 2u:{
                output[global_id.x+global_id.y*1280] = vec4(0,0,1,1);//blue
            }
            case 3u:{
                output[global_id.x+global_id.y*1280] = vec4(1,1,0,1);//yellow
            }
            case 4u:{
                output[global_id.x+global_id.y*1280] = vec4(1,0,1,1);//magenta
            }
            case 5u:{
                output[global_id.x+global_id.y*1280] = vec4(0,1,1,1);//cyan
            }
            case 6u:{
                output[global_id.x+global_id.y*1280] = vec4(1,1,1,1);//white
            }
            case 7u:{
                output[global_id.x+global_id.y*1280] = vec4(0,0,0,1);//black
            }
            default {

            }
        } 
 


        var header:u32 = octant.header[unmirrored_idx/2];
        if header!=0{
            output[global_id.x+global_id.y*1280] = vec4(.1,.1,.1,1);
            return;
        }
        if (unmirrored_idx&1)!=0u{
            header>>=16;
        }

        let is_child:bool = (header & 1)!=0;
        if(is_child){
                            output[global_id.x+global_id.y*1280] = vec4(1,0,1,1);

        }
        let is_leaf:bool =(header & (1<<8))!=0;

        if is_child && t_min<=t_max{

            if is_leaf&&t_min >=0.0{
                //hit

                let leaf_value:u32 = octant.data[unmirrored_idx];

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
                output[global_id.x+global_id.y*1280] = vec4(1.0,0,1,1);
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
                    parent_octant_idx = octant.data[unmirrored_idx];

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
        let t_corner_le_tc_max :vec3<bool> = t_corner < vec3(tc_max);
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
    return; //miss
}

/* // Assuming previous variables are defined:
// ro, rd, unmirrored_pos, t_corner, scale_exp2
// t_corner_eq_tc_min, rd_lt_0

// 1. Determine Mutually Exclusive Conditions
let cond0_active: bool = t_corner_eq_tc_min[0];
let cond1_active: bool = t_corner_eq_tc_min[1] && !cond0_active;
let cond2_active: bool = !(cond0_active || cond1_active);

// 2. Calculate face_id for All Three Cases
let sign_rd_0: u32 = bitcast<u32>(rd[0]) >> 31u;
let sign_rd_1: u32 = bitcast<u32>(rd[1]) >> 31u;
let sign_rd_2: u32 = bitcast<u32>(rd[2]) >> 31u;

let face_id_case0: u32 = (1u << 0u) | sign_rd_0;
let face_id_case1: u32 = (1u << 1u) | sign_rd_1;
let face_id_case2: u32 = (1u << 2u) | sign_rd_2;

// 3. Select the Final face_id
var face_id: u32 = face_id_case2; // Renamed from face_id_simd to match original
face_id = select(face_id, face_id_case1, cond1_active);
face_id = select(face_id, face_id_case0, cond0_active);

// 4. Calculate uv (before flip and scaling) for All Three Cases
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

// 5. Select and Scale uv
var uv_selected_raw = uv_raw_case2;
uv_selected_raw = select(uv_selected_raw, uv_raw_case1, cond1_active);
uv_selected_raw = select(uv_selected_raw, uv_raw_case0, cond0_active);

var uv: vec2<f32> = uv_selected_raw / scale_exp2; // Renamed from uv_simd

// 6. Handle Conditional uv Flipping
let flip_ux_cond: bool = (cond0_active && rd_lt_0[0]) || (cond2_active && rd_lt_0[2]);
uv.x = select(uv.x, 1.0 - uv.x, flip_ux_cond);

let flip_uy_cond: bool = cond1_active && rd_lt_0[1];
uv.y = select(uv.y, 1.0 - uv.y, flip_uy_cond); */