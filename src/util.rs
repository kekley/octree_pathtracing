use core::f32;
use std::f32::consts::PI;

use rand::{Rng, rngs::StdRng};

use glam::{Vec2, Vec3A};
use rand_distr::{Distribution, UnitDisc};

#[inline]
pub fn degrees_to_rads(degrees: f32) -> f32 {
    degrees * PI / 180.0
}

#[inline]
pub fn random_float(rng: &mut StdRng) -> f32 {
    rng.random::<f32>()
}

#[inline]
pub fn random_int(rng: &mut StdRng) -> i64 {
    random_float(rng) as i64
}

#[inline]
pub fn random_float_in_range(rng: &mut StdRng, min: f32, max: f32) -> f32 {
    min + (max - min) * random_float(rng)
}

#[inline]
pub fn linear_to_gamma(linear_component: f32) -> f32 {
    if linear_component > 0.0 {
        f32::sqrt(linear_component)
    } else {
        0.0
    }
}
#[inline]
pub fn random_vec(rng: &mut StdRng) -> Vec3A {
    Vec3A::new(random_float(rng), random_float(rng), random_float(rng))
}

#[inline]
pub fn random_vec_in_range(rng: &mut StdRng, min: f32, max: f32) -> Vec3A {
    Vec3A::new(
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
    )
}

#[inline]
pub fn random_unit_vec(rng: &mut StdRng) -> Vec3A {
    loop {
        let p = random_vec_in_range(rng, -1.0, 1.0);
        let len_sq = p.length_squared();

        if 1e-160 < len_sq && len_sq <= 1.0 {
            return p / f32::sqrt(len_sq);
        }
    }
}
#[inline]
pub fn random_on_hemisphere(rng: &mut StdRng, normal: Vec3A) -> Vec3A {
    let on_sphere = random_unit_vec(rng);
    if on_sphere.dot(normal) > 0.0 {
        on_sphere
    } else {
        -on_sphere
    }
}

#[inline]
pub fn step(edge: f32, x: f32) -> f32 {
    match x <= edge {
        true => 0.0,
        false => 1.0,
    }
}
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn step_vec(edge: f32, x: Vec3A) -> Vec3A {
    Vec3A::new(step(edge, x.x), step(edge, x.y), step(edge, x.z))
}

#[inline]
pub fn random_in_unit_disk(rng: &mut StdRng) -> Vec2 {
    let a: [f32; 2] = UnitDisc.sample(rng);
    Vec2::from_array(a)
}

pub fn near_zero(vec: &Vec3A) -> bool {
    let s = 1e-8;
    vec.x.abs() < s && vec.y.abs() < s && vec.z.abs() < s
}

pub fn defocus_disk_sample(rng: &mut StdRng, center: Vec3A, disc_u: Vec3A, disc_v: Vec3A) -> Vec3A {
    let p = random_in_unit_disk(rng);
    center + (p.x * disc_u) + (p.y * disc_v)
}

#[inline]
pub fn sample_square(rng: &mut StdRng) -> Vec2 {
    Vec2::new(random_float(rng) - 0.5, random_float(rng) - 0.5)
}
pub fn find_msb(mut x: i32) -> i32 {
    let mut res = -1;
    if x < 0 {
        x = !x;
    }
    for i in 0..32 {
        let mask = 0x80000000u32 as i32 >> i;
        if x & mask != 0 {
            res = 31 - i;
            break;
        }
    }
    res
}
pub fn find_msb_u32(x: u32) -> u32 {
    // Decide what to do when x is zero.
    // One common strategy is to define the msb of 0 as 0.
    if x == 0 {
        return -1i32 as u32;
    }
    // The bit-length of a u32 is 32 bits.
    // The built-in function `leading_zeros()` returns the number of zeros from the most significant bit down to the first 1.
    // For example, if x is 16 (0b0001_0000), then x.leading_zeros() returns 27.
    // Since the highest index in a 32-bit number is 31, subtracting gives us:
    //   31 - 27 = 4, which is indeed the index of the most significant bit (since 16 == 2^4).
    31 - x.leading_zeros()
}
pub fn angle_distance(a1: f32, a2: f32) -> f32 {
    let diff = (a1 - a2).abs() % (2.0 * PI);
    if diff > PI { 2.0 * PI - diff } else { diff }
}

const NUM_U32_WORDS: usize = 8;
const BITS_PER_U32_WORD: usize = 32;
const TOTAL_BITS_IN_ARRAY: usize = NUM_U32_WORDS * BITS_PER_U32_WORD; // 8 * 32 = 256 bits
const BITS_PER_CHUNK: usize = 30;
const MAX_VALID_START_BIT: usize = TOTAL_BITS_IN_ARRAY - BITS_PER_CHUNK; // 256 - 30 = 226

/// Extracts a 30-bit integer from an array of 8 u32 values.
///
/// The input array `arr` represents a continuous bitstream of 8 * 32 = 256 bits.
/// This function extracts a 30-bit chunk starting at `starting_bit_abs`.
///
/// # Arguments
/// * `arr`: A reference to a fixed-size array of 8 `u32` values.
/// * `starting_bit_abs`: The absolute 0-based starting bit position of the
///   30-bit chunk to extract in the 256-bit stream (0-indexed from MSB of arr[0]).
///
/// # Returns
/// A `u32` containing the extracted 30-bit integer in its lower bits.
///
/// # Panics
/// Panics if `starting_bit_abs` would cause the 30-bit chunk to exceed array bounds.
pub fn extract_u30_from_u32_arr(arr: &[u32; NUM_U32_WORDS], starting_bit_abs: usize) -> u32 {
    const MASK_30_BITS_U64: u64 = (1u64 << BITS_PER_CHUNK) - 1;

    if starting_bit_abs > MAX_VALID_START_BIT {
        panic!(
            "starting_bit_abs {} is out of bounds. Max valid start bit for a {}-bit chunk in a {}-bit array is {}.",
            starting_bit_abs, BITS_PER_CHUNK, TOTAL_BITS_IN_ARRAY, MAX_VALID_START_BIT
        );
    }

    // Determine the index of the u32 word in `arr` that contains the `starting_bit_abs`.
    let word_idx = starting_bit_abs / BITS_PER_U32_WORD;
    // Determine the bit offset within `arr[word_idx]` where our 30-bit sequence starts.
    // This offset is from the MSB of arr[word_idx].
    let bit_offset_in_word = starting_bit_abs % BITS_PER_U32_WORD;

    // Load up to two u32 words from the array.
    // These words form a 64-bit window (2 * 32 bits) that is guaranteed
    // to contain the desired 30-bit chunk.
    let u0 = arr[word_idx] as u64;
    let u1 = if word_idx + 1 < NUM_U32_WORDS {
        arr[word_idx + 1] as u64
    } else {
        0 // If the chunk is entirely within the last u32, this part is not used.
    };

    // Combine these two u32 values into a 64-bit number.
    // The layout in combined_val_u64 (from MSB to LSB of the 64 bits used):
    // [ u0 (32 bits) | u1 (32 bits) ]
    // MSB of u0 corresponds to bit 63 of this 64-bit segment, LSB of u1 is bit 0.
    let combined_val_u64: u64 = (u0 << BITS_PER_U32_WORD) | u1;

    // The 30-bit sequence starts `bit_offset_in_word` bits from the MSB of u0's portion.
    // u0's MSB is at bit 63 of `combined_val_u64`.
    // So, the MSB of our 30-bit sequence is at bit `63 - bit_offset_in_word` in `combined_val_u64`.
    // The LSB of our 30-bit sequence is at bit `(63 - bit_offset_in_word) - (BITS_PER_CHUNK - 1)`.
    // To align this LSB to bit 0 of the result, we need to shift `combined_val_u64` right by this amount.
    // right_shift_amount = (63 - bit_offset_in_word) - (BITS_PER_CHUNK - 1)
    //                    = 63 - bit_offset_in_word - BITS_PER_CHUNK + 1
    //                    = (BITS_PER_U32_WORD * 2) - 1 - bit_offset_in_word - BITS_PER_CHUNK + 1
    //                    = (BITS_PER_U32_WORD * 2) - bit_offset_in_word - BITS_PER_CHUNK
    // This is equivalent to: total_window_bits - (offset_from_MSB_of_window + chunk_size)
    let right_shift_amount = (BITS_PER_U32_WORD * 2) - (bit_offset_in_word + BITS_PER_CHUNK);

    // Shift to align the desired 30 bits to the LSB.
    let extracted_val_u64 = combined_val_u64 >> right_shift_amount;

    // Apply a mask to ensure only the lower 30 bits are kept.
    (extracted_val_u64 & MASK_30_BITS_U64) as u32
}

pub fn write_u30_to_u32_arr(
    starting_bit_abs: usize,
    value_to_write: u32,
    array: &mut [u32; NUM_U32_WORDS],
) {
    const MASK_30_BITS_U64: u64 = (1u64 << BITS_PER_CHUNK) - 1;
    const U32_MASK_U64: u64 = 0xFFFFFFFFu64;

    if starting_bit_abs > MAX_VALID_START_BIT {
        panic!(
            "starting_bit_abs {} is out of bounds. Max valid start bit for a {}-bit chunk in a {}-bit array is {}.",
            starting_bit_abs, BITS_PER_CHUNK, TOTAL_BITS_IN_ARRAY, MAX_VALID_START_BIT
        );
    }
    if value_to_write >= (1u32 << BITS_PER_CHUNK) {
        panic!(
            "Value {} is too large to fit in {} bits.",
            value_to_write, BITS_PER_CHUNK
        );
    }

    // Determine the index of the u32 word in `array` that contains this `starting_bit_abs`.
    let word_idx = starting_bit_abs / BITS_PER_U32_WORD;
    // Determine the bit offset within `array[word_idx]` where our 30-bit sequence starts.
    let bit_offset_in_word = starting_bit_abs % BITS_PER_U32_WORD;

    // The amount to shift the 30-bit value left to position it correctly within a 64-bit window.
    // This calculation is symmetrical to `right_shift_amount` in `extract_u30_from_u32_arr`.
    let window_shift_amount = (BITS_PER_U32_WORD * 2) - (bit_offset_in_word + BITS_PER_CHUNK);

    // Position the 30-bit value_to_write correctly within a 64-bit integer,
    // as if it were part of the 64-bit conceptual window.
    let data_to_write_shifted: u64 = (value_to_write as u64) << window_shift_amount;

    // Create a mask for the 30-bit slot within the 64-bit window.
    let slot_mask_in_window: u64 = MASK_30_BITS_U64 << window_shift_amount;

    // Load the existing u32 words that will be affected by this write.
    // These form the current 64-bit window.
    let u0_current = array[word_idx] as u64;
    let u1_current = if word_idx + 1 < NUM_U32_WORDS {
        array[word_idx + 1] as u64
    } else {
        0
    };
    let current_data_in_window: u64 = (u0_current << BITS_PER_U32_WORD) | u1_current;

    // Clear the bits in the 30-bit slot within the loaded window.
    let cleared_data_in_window = current_data_in_window & !slot_mask_in_window;

    // Combine the cleared window with the new data to be written.
    let new_data_in_window = cleared_data_in_window | data_to_write_shifted;

    // Write the updated u32 values back to the array.
    array[word_idx] = (new_data_in_window >> BITS_PER_U32_WORD) as u32;
    if word_idx + 1 < NUM_U32_WORDS {
        array[word_idx + 1] = (new_data_in_window & U32_MASK_U64) as u32;
    }
}
