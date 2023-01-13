use crate::OutputBitStream;
use crate::{Encode, LEADING_REPR_ENC, LEADING_ROUND};

// Chimp N (= 128)

const THRESHOLD: usize = 13;
const SET_LSB: usize = 0x3FFF;

struct Encoder {
    first: bool,
    stored_vals: Vec<u64>,
    indices: Vec<usize>,

    leading_zeros: u32,
    curr_idx: usize,
    index: usize, // always points to previous index
    w: OutputBitStream,

    size: u64, // for testing
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            first: true,
            stored_vals: Vec::with_capacity(128),
            indices: Vec::with_capacity(2_usize.pow(14)),
            leading_zeros: 0,
            curr_idx: 0,
            index: 0,
            w: OutputBitStream::new(),
            size: 0,
        }
    }

    fn insert_first(&mut self, value: f64) {
        self.stored_vals[self.index] = value.to_bits();
        self.indices[value.to_bits() as usize & SET_LSB] = self.index;

        self.w.write_bits(value.to_bits(), 64);
    }

    fn insert_value(&mut self, value: f64) {
        let lsb_index = self.indices[(value.to_bits() as usize & SET_LSB)];

        let prev_index: usize;
        let mut trail: u32 = 0;
        let mut xor: u64;

        // if value with same lsb is still in scope
        if (self.index - lsb_index) < 128 {
            xor = value.to_bits() ^ self.stored_vals[lsb_index % 128];
            trail = xor.trailing_zeros();

            if trail > THRESHOLD as u32 {
                // very similar values, so we use prev_index from indices
                prev_index = lsb_index % 128;
            } else {
                // previous value
                prev_index = self.index % 128;
                xor = self.stored_vals[self.index] ^ value.to_bits();
            }
        } else {
            prev_index = self.index % 128;
            xor = self.stored_vals[self.index] ^ value.to_bits();
        }

        // identical value
        if xor == 0 {
            self.w.write_bits(prev_index as u64, 9); // 'flagZeroSize' = log_2(ring_buffer_size) + 2
            self.leading_zeros = 65;
        } else {
            let lead = LEADING_ROUND[xor.leading_zeros() as usize];

            if trail > THRESHOLD as u32 {
                let center_bits = u64::from(64 - lead - trail);

                let tmp = 512 * (128 + prev_index as u64)
                    + (64 * LEADING_REPR_ENC[lead as usize] as u64)
                    + center_bits;

                self.w.write_bits(tmp, 18); // flagOneSize = log_2(ring_buffer_size) + 11
                self.w.write_bits(xor >> trail, center_bits as u32);

                self.leading_zeros = 65;
            } else {
                let center_bits = 64 - lead;

                if lead != self.leading_zeros {
                    self.leading_zeros = lead;
                    self.w
                        .write_bits(24 + LEADING_REPR_ENC[lead as usize] as u64, 5)
                } else {
                    self.w.write_bits(2, 2);
                }

                self.w.write_bits(xor, center_bits);
            }
        }

        self.curr_idx += 1;
        self.curr_idx %= 128;

        self.stored_vals[self.curr_idx] = value.to_bits();

        self.index += 1;
        self.indices[value.to_bits() as usize & SET_LSB] = self.index;
    }
}

impl Encode for Encoder {
    fn encode(&mut self, value: f64) {
        if self.first {
            self.first = false;
            self.insert_first(value);
        } else {
            self.insert_value(value);
        }
    }

    fn close(&mut self) -> Box<[u8]> {
        self.insert_value(f64::NAN);
        self.w.clone().close() // TODO: wtf
    }
}
