

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

struct BlockModel{
    ind:u32,
    len:u32,
}

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



@group(0) @binding(0)
var<uniform> context : TraversalContext;


@group(1) @binding(0)
var<storage,read> octree: array<u32>;
@group(1) @binding(1)
var<storage,read> models: array<BlockModel>;
@group(1) @binding(2)
var<storage,read> quads: array<GPUQuad>;
@group(1) @binding(3)
var<storage,read> materials: array<GPUMaterial>;
@group(1) @binding(4)
var textures : binding_array<texture_2d<u32>>;
@group(1) @binding(5)
var nearest_sampler: sampler;
@group(1) @binding(6)
var output: texture_storage_2d<rgba8unorm,write>;


const OCTREE_MAX_SCALE:u32 =23;
const OCTREE_MAX_STEPS:u32 = 1000;
const OCTREE_EPSILON:f32 = 1.1920929e-7;
const RAY_EPSILON:f32 = 5e-8;

const camera = CameraParams(vec3(1.0,200.0,20.0),vec3(100.0,130,100.0),vec3(0.0,1.0,0.0),70.0,16.0/9.0);

const WORKGROUP_SIZE_X = 8u;
const WORKGROUP_SIZE_Y = 8u;
const WORKGROUP_SIZE_TOTAL = WORKGROUP_SIZE_X*WORKGROUP_SIZE_Y;

@compute @workgroup_size(WORKGROUP_SIZE_X,WORKGROUP_SIZE_Y,1)
fn main(@builtin(local_invocation_index) local_idx: u32,@builtin(global_invocation_id) global_id: vec3<u32>) {
    let octree_length = arrayLength(&octree);
    if global_id.x >= 1280 || global_id.y >= 720 {
        return;
    }
    
    let ray = create_ray_optimized(camera,global_id.xy);
    rays[local_idx] = ray;
    intersect_octree(global_id,local_idx,1024.0);
}

const MAX_BOUNCES:u32 = 5;
const FIRST_RAY_REUSE_COUNT:u32 =20;

fn trace_ray()->vec3<f32>{
    var accumulated_color = vec3<f32>(0.0);
    var outer_attenuation = vec3<f32>(1.0);
    let hit:bool=false;
    //test if hit

    //if hit then we use this same ray multiple times

    if hit{
        
        // accumulated_color += outer_attenuation * get_emission(first_hit_data);
        
        //update attenuation here

        for(var branch_count:u32=0u; branch_count < FIRST_RAY_REUSE_COUNT; branch_count+=1u){
            //do reflection calculation based on material data and randomness
            //new ray created here, outer attenuation updated
            var branch_ray:Ray;
            var branch_attenuation:vec3<f32> = outer_attenuation; // *bsdf/pdf
            var bounce_count:u32 = 0u;
            var branch_color:vec3<f32> = vec3<f32>(0.0);
            //cast branch ray, test for hit
            //let branch_hit_data = inner_trace_ray(branch_ray);
            // if branch_hit_data.hit;
                //path taken by a branch
                while bounce_count<MAX_BOUNCES{
                    //cast branch ray
                    if true{ // if we hit
                    //Update branch_attenuation based on material interaction:
                    //let bsdf_factor_and_pdf = evaluate_material_and_sample_direction(branch_hit_data);
                    //branch_attenuation *= bsdf_factor_and_pdf.value / bsdf_factor_and_pdf.pdf;
                    //branch_ray = reflected ray
                    
                    //check for absorption? (russian roulette or attenuation is too low)
                    //if absorption{break;}

                    bounce_count++;
                    }else{
                        //branch ray missed
                        //branch_color += branch_attenuation * sky_color
                        break;
                    }
                }

            //accumulated_color+=branch_color;
        }
    
    }else{
            //first ray missed, do sky calculation
            //sky_color = sky(ray);
            //accumulated_color+=current_attenuation*sky_color
    }
    return accumulated_color;
}

fn max_vec3(v: vec3<f32> )->f32{
    return max(v.x,max(v.y,v.z));
}

fn min_vec3(v: vec3<f32> )->f32{
    return min(v.x,min(v.y,v.z));
}



var<workgroup> time_stacks: array<array<f32,24>,WORKGROUP_SIZE_TOTAL>;
var<workgroup> octant_stacks: array<array<u32,24>,WORKGROUP_SIZE_TOTAL>;
var<workgroup> rays: array<Ray,WORKGROUP_SIZE_TOTAL>;

fn intersect_quad(ray:ptr<function,Ray>,quad:ptr<function,GPUQuad>,voxel_position:vec3<f32>,t_next:f32)->bool{
    let translated_ray_origin = (*ray).origin-voxel_position;
    let denominator = dot((*ray).direction,(*quad).normal);

    if denominator>=-RAY_EPSILON{
        return false;
    }

    let t = ((*quad).plane_d- dot((*quad).normal,translated_ray_origin))/denominator;
    
    if t<=0.0 || t> t_next{
        return false;
    }

    let intersection = translated_ray_origin + (*ray).direction *t;

    let planar_hit_point = intersection- (*quad).origin;

    let n = cross((*quad).u,(*quad).v);

    let w = n / dot(n,n);

    let alpha = dot(w, cross(planar_hit_point,(*quad).v));

    let beta = dot(w,cross((*quad).u,planar_hit_point));


    if alpha < 0.0 || alpha > 1.0 || beta < 0.0 || beta > 1.0 {
            return false;
    }
    
    
    return true;
}

struct OctreeIntersectResult{
    material_id:u32,

}

fn intersect_octree(global_id:vec3<u32>,local_idx:u32, max_dst: f32) {
    let octree_scale: f32 = context.octree_scale;
    var root: u32 = context.root;
    var scale: u32 = context.scale;
    let octant_stack :ptr<workgroup,array<u32,24>> = &octant_stacks[local_idx];
    let time_stack : ptr<workgroup,array<f32,24>> = &time_stacks[local_idx];
    var ro: vec3<f32> =rays[local_idx].origin;
    ro*=octree_scale;
    ro += 1.0;
    var rd: vec3<f32> = rays[local_idx].direction;
    var scale_exp2: f32 = exp2(f32(i32(scale) - i32(OCTREE_MAX_SCALE)));
    var parent_octant_idx: u32 = root;

    var sign_mask: u32 = 1u << 31u;

    let epsilon_bits_without_sign: u32 = bitcast<u32>(OCTREE_EPSILON) & (~sign_mask);
    


    let dir_lt_epsilon: vec3<bool> = abs(rd) < vec3(OCTREE_EPSILON);

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

    var idx:u32 = 0u;

    var pos : vec3<f32> = vec3(1.0);
    let upper:vec3<f32> = 1.5*t_coef - t_bias;

    let lt_upper : vec3<bool> =  vec3(t_min) < upper;

    idx ^= vec_to_bitmask(lt_upper);
    pos = select(pos,vec3(1.5),lt_upper);

    for(var i:u32=0;i<1024;i++){
        if max_dst>=0.0 && t_min>max_dst{
            //miss
        }


        let t_corner :vec3<f32> = pos*t_coef-t_bias;

        let tc_max:f32 = min_vec3(t_corner);

        let unmirrored_idx:u32 = idx^mirror_mask;
        

        let current_node_base_ptr = 12u * parent_octant_idx;
        let header_word_containing_child_header = octree[current_node_base_ptr + (unmirrored_idx / 2u)];
        let shift_for_child_header = 16u * (unmirrored_idx % 2u);
        let header_16bit: u32 = (header_word_containing_child_header >> shift_for_child_header) & 0xFFFFu; // Isolate the 16 bits

        let is_child:bool = (header_16bit & 255u) != 0u; // Now checks lower 8 bits of the 16-bit header
        let is_leaf:bool = (header_16bit == 0xFFFFu);   // Now correctly checks if the 16-bit header is 0xFFFF

        if is_child && t_min<=t_max{

            if is_leaf&&t_min >=0.0{
                //hit
                let leaf_value:u32 = octree[12*parent_octant_idx+4+unmirrored_idx];

                let unmirrored_components:vec3<f32> = 3.0-scale_exp2-pos;
                
                let unmirror_bools :vec3<bool> = bitmask_to_vec(mirror_mask);
                
                let unmirrored_pos = select(pos,unmirrored_components,unmirror_bools);


                let t_corner:vec3<f32> = (pos+scale_exp2)*t_coef-t_bias;

                let tc_min = max_vec3(t_corner);

                
                let t_corner_eq_tc_min:vec3<bool> = t_corner==vec3(tc_min);

                let rd_lt_0 : vec3<bool> = rd<vec3(0.0);
                let cond0_active: bool = t_corner_eq_tc_min.x;
                let cond1_active: bool = t_corner_eq_tc_min.y && !cond0_active;
                let cond2_active: bool = !(cond0_active || cond1_active);

                let sign_rd_0: u32 = bitcast<u32>(rd.x) >> 31u;
                let sign_rd_1: u32 = bitcast<u32>(rd.y) >> 31u;
                let sign_rd_2: u32 = bitcast<u32>(rd.z) >> 31u;

                let face_id_case0: u32 = (1u << 0u) | sign_rd_0;
                let face_id_case1: u32 = (1u << 1u) | sign_rd_1;
                let face_id_case2: u32 = (1u << 2u) | sign_rd_2;

                var face_id: u32 = face_id_case2; 
                face_id = select(face_id, face_id_case1, cond1_active);
                face_id = select(face_id, face_id_case0, cond0_active);

                let uv_raw_case0 = vec2<f32>(
                    (ro.z + rd.z * t_corner.x) - unmirrored_pos.z,
                    (ro.y + rd.y * t_corner.x) - unmirrored_pos.y
                );
                let uv_raw_case1 = vec2<f32>(
                    (ro.x + rd.x * t_corner.y) - unmirrored_pos.x,
                    (ro.z + rd.z * t_corner.y) - unmirrored_pos.z
                );
                let uv_raw_case2 = vec2<f32>(
                    (ro.x + rd.x * t_corner.z) - unmirrored_pos.x,
                    (ro.y + rd.y * t_corner.z) - unmirrored_pos.y
                );

                var uv_selected_raw = uv_raw_case2;
                uv_selected_raw = select(uv_selected_raw, uv_raw_case1, cond1_active);
                uv_selected_raw = select(uv_selected_raw, uv_raw_case0, cond0_active);

                var uv: vec2<f32> = uv_selected_raw / scale_exp2; // Renamed from uv_simd

                let flip_ux_cond: bool = (cond0_active && rd_lt_0.x) || (cond2_active && rd_lt_0.z);
                uv.x = select(uv.x, 1.0 - uv.x, flip_ux_cond);

                let flip_uy_cond: bool = cond1_active && rd_lt_0.y;
                uv.y = select(uv.y, 1.0 - uv.y, flip_uy_cond); 
                //hit

                let quad = quads[leaf_value];
                let material = materials[quad.material_id];
                let texture_id = material.texture_index;
                
                let texture = textures[texture_id];
                let uv_2 :vec2<u32> = vec2<u32>(uv*16);

                let color:vec4<u32> = textureLoad(texture,uv_2,0i);

                let float_color = vec4<f32>(f32(color.x),f32(color.y),f32(color.z),f32(color.w)) / 255.0;

                textureStore(output,global_id.xy,float_color);
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
                        (*octant_stack)[scale] = parent_octant_idx;
                        (*time_stack)[scale] = t_max;
                    }

                    h = tc_max;
                    //get the new octant value
                    parent_octant_idx = octree[12*parent_octant_idx+4+unmirrored_idx];


                    scale-=1u;
                    scale_exp2 = half_scale;

                    idx=0u;
                    if t_min<t_center.x{
                        idx^=1u;
                        pos.x +=scale_exp2;
                    }
                    if t_min<t_center.y{
                        idx^=2u;
                        pos.y +=scale_exp2;
                    }
                    if t_min<t_center.z{
                        idx^=4u;
                        pos.z +=scale_exp2;
                    }


                    t_max = tv_max;
                    continue;
                }
            }
        }
        //advance step

        //calculate how to step child index
        var step_mask:u32 = 0u;
        if tc_max>=t_corner.x{
            step_mask^=1u;
            pos.x-=scale_exp2;
        }

        if tc_max>=t_corner.y{
            step_mask^=2u;
            pos.y-=scale_exp2;
        }

        if tc_max>=t_corner.z{
            step_mask^=4u;
            pos.z-=scale_exp2;
        }

        t_min = tc_max;
        idx^=step_mask;

        if (idx&step_mask)!=0{
            //pop step


   
           var differing_bits:u32 = 0u;


            if (step_mask&1u)!=0{
                differing_bits|= bitcast<u32>(pos.x) ^ bitcast<u32>(pos.x+scale_exp2);
            }
            if (step_mask&2u)!=0{
                differing_bits|= bitcast<u32>(pos.y) ^ bitcast<u32>(pos.y+scale_exp2);
            }
            if (step_mask&4u)!=0{
                differing_bits|= bitcast<u32>(pos.z) ^ bitcast<u32>(pos.z+scale_exp2);
            }

            scale = firstLeadingBit(differing_bits);

            scale_exp2 = exp2(f32(i32(scale)-i32(OCTREE_MAX_SCALE)));

            if scale>=OCTREE_MAX_SCALE{
                return; //miss
            }

            parent_octant_idx = (*octant_stack)[scale];
            t_max = (*time_stack)[scale];

            let shifted_pos = bitcast<u32>(pos) >> vec3(scale);
            pos = bitcast<f32>(shifted_pos<<vec3(scale));

            idx = (bitcast<u32>(shifted_pos.x)&1) | (bitcast<u32>(shifted_pos.y)&1) <<1 | (bitcast<u32>(shifted_pos.z)&1) <<2;
            h=0.0;
        }
    }
    return; //miss
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