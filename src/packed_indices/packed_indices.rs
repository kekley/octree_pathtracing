pub struct PackedIndices {
    bits_per_index: u8,
    values_per_long: u8,
    index: usize,
    buffer: Vec<u64>,
}

impl PackedIndices {
    pub fn new(bits_per_index: u8) -> PackedIndices {
        let values_per_long = u64::BITS / bits_per_index as u32;
        PackedIndices {
            bits_per_index,
            values_per_long: values_per_long as u8,
            index: 0,
            buffer: vec![],
        }
    }
    #[inline]
    pub fn write_packed_index(&mut self, value: u64) {
        let values_per_long = u64::BITS / self.bits_per_index as u32;

        let long_index = self.index / values_per_long as usize;
        if long_index <= self.buffer.len() {
            self.buffer.push(0);
        }
        let shift_amount = self.bits_per_index as usize * (self.index % values_per_long as usize);

        let shifted = value << shift_amount;

        if let Some(long) = self.buffer.get_mut(long_index) {
            *long |= shifted;
        } else {
            panic!();
        }
        self.index += 1;
    }
    #[inline]
    pub fn reset(&mut self, bits_per_index: u8) {
        self.bits_per_index = bits_per_index;

        let values_per_long = u64::BITS / bits_per_index as u32;

        self.values_per_long = values_per_long as u8;

        self.buffer.clear();
        self.index = 0;
    }

    #[inline]
    pub fn read_packed_index(&self, index: usize) -> Option<u64> {
        if index > self.index {
            return None;
        }
        let long_index = index / self.values_per_long as usize;
        let shift_right_amount: u32 =
            self.bits_per_index as u32 * (index as u32 % self.values_per_long as u32);
        let shift_left_amount: u32 = u64::BITS - self.bits_per_index as u32 - shift_right_amount;

        let long = *self.buffer.get(long_index)?;

        let shifted = (long << shift_left_amount) >> (shift_right_amount + shift_left_amount);

        Some(shifted)
    }
}
