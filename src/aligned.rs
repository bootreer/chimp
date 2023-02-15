use crate::chimpn::{LSB_MASK, THRESHOLD};
use crate::*;
// since chimp compression doesn't guarantee byte alignedness,
// added this to have decode and encode perform better

// Based off of the Patas compression implemented in DuckDB
#[derive(Debug)]
pub struct Encoder {
    first: bool,
    leading_zeros: u32,
    w: OutputBitStream,
    pub size: u64,
    curr_idx: usize,
    index: usize,

    stored_vals: Vec<u64>, // since Chimp128 offers close to 50% compression
    indices: Vec<usize>,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            first: true,
            stored_vals: vec![0; 128],
            indices: vec![usize::MAX; 2_usize.pow(14)],
            leading_zeros: 0,
            curr_idx: 0,
            index: 0,
            w: OutputBitStream::new(),
            size: 0,
        }
    }

    fn insert_first(&mut self, value: f64) {
        self.stored_vals[self.index] = value.to_bits();
        self.indices[value.to_bits() as usize & LSB_MASK] = self.index;

        self.w.write_bits(value.to_bits(), 64);

        self.size += 64;
    }

    // TODO: fix this boy
    #[allow(unused_variables, unused_mut)]
    fn insert_value(&mut self, value: f64) {
        let prev_index: usize;
        let lead: u32;
        let trail: u32;
        let mut xor: u64;

        let mut lsb_index = self.indices[(value.to_bits() as usize & LSB_MASK)];
        // not sure about the first condition
        if lsb_index > 128 || (self.index - lsb_index) >= 128 {
            lsb_index = 128; // ???
        }
        let ref_value = self.stored_vals[lsb_index % 128];

        xor = ref_value ^ value.to_bits();
        trail = xor.trailing_zeros();
        lead = xor.leading_zeros();

        let is_equal = if xor == 0 { 0 } else { 1 };
        let sig_bits = if xor == 0 { 0 } else { 64 - trail - lead };
        // let sig_bytes = (sig_bits >> 3) + if sig_bits & 7 != 0 { 1 } else { 0 };

        self.w.write_bits(xor.wrapping_shl(trail - is_equal), sig_bits);
        self.size += sig_bits as u64;

        self.curr_idx += 1;
        self.curr_idx %= 128;

        self.stored_vals[self.curr_idx] = value.to_bits();

        self.index += 1;
        self.indices[value.to_bits() as usize & LSB_MASK] = self.index;
    }

    pub fn insert(&mut self, value: f64) { 
        if self.first {
            self.insert_first(value);
            self.first = false;
        } else {
            self.insert_value(value);
        }
    }

}
