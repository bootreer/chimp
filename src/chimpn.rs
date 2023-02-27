use crate::*;

// Chimp N (= 128)
pub const THRESHOLD: usize = 13;
pub const LSB_MASK: usize = 0x3FFF;

pub struct Encoder {
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
            stored_vals: vec![0; 128],
            indices: vec![0; 2_usize.pow(14)],
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

    fn insert_value(&mut self, value: f64) {
        let prev_index: usize;
        let mut trail: u32 = 0;
        let mut xor: u64;

        let lsb_index = self.indices[value.to_bits() as usize & LSB_MASK];

        // if value with same lsb is still in scope
        if (self.index - lsb_index) < 128 { // in lower numbers likely 0?
            xor = value.to_bits() ^ self.stored_vals[lsb_index % 128];
            trail = xor.trailing_zeros();

            if trail > THRESHOLD as u32 {
                // very similar values, so we use prev_index from indices
                prev_index = lsb_index % 128;
            } else {
                // previous value
                prev_index = self.index % 128;
                xor = self.stored_vals[self.curr_idx] ^ value.to_bits();
            }
        } else {
            prev_index = self.index % 128;
            xor = self.stored_vals[self.curr_idx] ^ value.to_bits();
        }

        // identical value
        // flag: 00
        if xor == 0 {
            self.w.write_bits(prev_index as u64, 9); // 'flagZeroSize' = log_2(ring_buffer_size) + 2
            self.leading_zeros = 65;

            self.size += 9;
        } else {
            let lead = LEADING_ROUND[xor.leading_zeros() as usize];

            // flag: 01
            if trail > THRESHOLD as u32 {
                let center_bits = u64::from(64 - lead - trail);

                let tmp = (128 + prev_index as u64) << 9
                    | (LEADING_REPR_ENC[lead as usize] as u64) << 6
                    | center_bits;

                self.w.write_bits(tmp, 18); // flagOneSize = log_2(ring_buffer_size) + 11
                self.w.write_bits(xor >> trail, center_bits as u32);

                self.leading_zeros = 65;

                self.size += 18 + center_bits;
            } else {
                let center_bits = 64 - lead;

                if lead != self.leading_zeros {
                    self.leading_zeros = lead;

                    self.w.write_bits(3, 2); // flag: 11
                    self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3)
                } else {
                    self.w.write_bits(2, 2); // flag: 10
                }

                self.w.write_bits(xor, center_bits);

                self.size += 3 + center_bits as u64;
            }
        }

        self.curr_idx += 1;
        self.curr_idx %= 128;

        self.stored_vals[self.curr_idx] = value.to_bits();

        self.index += 1;
        self.indices[value.to_bits() as usize & LSB_MASK] = self.index;
    }
}

impl Encode for Encoder {
    fn encode_vec(values: &Vec<f64>) -> Self {
        let mut chimpn = Encoder {
            first: true,
            stored_vals: vec![0; 128],
            indices: vec![0; 2_usize.pow(14)],
            leading_zeros: 0,
            curr_idx: 0,
            index: 0,
            w: OutputBitStream::with_capacity(values.len()/2),
            size: 0,
        };
        for &val in values {
            chimpn.encode(val);
        }
        chimpn
    }

    fn encode(&mut self, value: f64) {
        if self.first {
            self.first = false;
            self.insert_first(value);
        } else {
            self.insert_value(value);
        }
    }

    fn close(self) -> (Box<[u64]>, u64) {
        let mut this = self;
        this.insert_value(f64::NAN);
        this.w.write_bit(0); // not sure why actual implementation does this
        (this.w.close(), this.size) // TODO: wtf
    }
}

pub struct Decoder {
    first: bool,
    done: bool,

    stored_vals: Vec<u64>,
    curr: u64, // curr stored value
    curr_idx: usize,
    leading_zeros: u32,
    trailing_zeros: u32,
    r: InputBitStream,
}

// prev_values = 128
// prev_values_log = 7
// initial_fill = 7 + 9 = 16

impl Decoder {
    pub fn new(r: InputBitStream) -> Self {
        Decoder {
            first: true,
            done: false,
            stored_vals: (0..128).collect(),
            curr: 0,
            curr_idx: 0,
            leading_zeros: u32::MAX,
            trailing_zeros: 0,
            r,
        }
    }

    fn get_first(&mut self) -> Result<(), Error> {
        self.curr = self.r.read_bits(64)?;
        self.stored_vals[self.curr_idx] = self.curr;
        Ok(())
    }

    fn get_value(&mut self) -> Result<(), Error> {
        let xor: u64;

        match self.r.read_bits(2)? {
            1 => {
                // prev_values = 128
                // prev_values_log = 7
                // initial_fill = 7 + 9 = 16
                let mut tmp = self.r.read_bits(16)?;
                let mut center_bits = tmp & 0x3F;
                tmp >>= 6;

                self.leading_zeros = LEADING_REPR_DEC[(tmp & 7) as usize];
                tmp >>= 3;

                let index = tmp & ((1 << 7) - 1);
                self.curr = self.stored_vals[index as usize];

                if center_bits == 0 {
                    center_bits = 64;
                }

                self.trailing_zeros = 64 - center_bits as u32 - self.leading_zeros;
                xor = self.r.read_bits(center_bits as u32)?;
                self.curr ^= xor << self.trailing_zeros;
            }
            2 => {
                xor = self.r.read_bits(64 - self.leading_zeros)?;
                self.curr ^= xor;
            }
            3 => {
                self.leading_zeros = LEADING_REPR_DEC[self.r.read_bits(3)? as usize];
                xor = self.r.read_bits(64 - self.leading_zeros)?;
                self.curr ^= xor;
            }
            _ => {
                let index = self.r.read_bits(7)? as usize;
                self.curr = self.stored_vals[index];
            }
        }

        self.curr_idx += 1;
        self.curr_idx %= 128;
        self.stored_vals[self.curr_idx] = self.curr;

        Ok(())
    }

    pub fn get_next(&mut self) -> Result<u64, Error> {
        if self.done {
            return Err(Error::EOF);
        }

        if self.first {
            self.get_first()?;
            self.first = false;
        } else {
            self.get_value()?;
        }

        if self.curr == NAN {
            Err(Error::EOF)
        } else {
            Ok(self.curr)
        }
    }
}

impl Iterator for Decoder {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.first {
            self.first = false;
            self.get_first().unwrap();
        } else {
            self.get_value().unwrap();
        }

        if self.curr == NAN {
            self.done = true;
            None
        } else {
            Some(self.curr)
        }
    }
}

#[cfg(test)]
mod chimp_tests {
    use super::{Decoder, Encoder};
    use crate::bitstream::InputBitStream;
    use crate::Encode;

    #[test]
    fn simple_test() {
        let float_vec: Vec<f64> = [
            49.4, 48.8, 46.4, 47.9, 48.7, 48.9, 48.8, 46.4, 47.9, 48.7, 48.9,
        ]
        .to_vec();

        let mut encoder = Encoder::new();

        for val in &float_vec {
            encoder.encode(*val);
        }

        let (bytes, _) = encoder.close();
        let mut decoder = Decoder::new(InputBitStream::new(bytes));
        let mut datapoints = Vec::new();

        while let Ok(val) = decoder.get_next() {
            datapoints.push(f64::from_bits(val));
        }

        assert_eq!(datapoints, float_vec);
    }
}
