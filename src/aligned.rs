use crate::chimpn::{LSB_MASK, THRESHOLD};
use crate::*;
// since chimp compression doesn't guarantee byte alignedness,
// added this to have decode and encode perform better

// Based off of the Patas compression implemented in DuckDB
// need to fix close? and test some edge cases for encoding
#[derive(Debug)]
pub struct Encoder {
    first: bool,
    pub w: OutputBitStream,
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
            curr_idx: 0,
            index: 0,
            w: OutputBitStream::new(),
        }
    }

    fn insert_first(&mut self, value: f64) {
        self.stored_vals[self.index] = value.to_bits();
        self.indices[(value.to_bits() & LSB_MASK) as usize] = self.index;

        self.w.write_bits(value.to_bits(), 64);
    }

    // TODO: fix this boy
    fn insert_value(&mut self, value: f64) {
        let mut lsb_index = self.indices[(value.to_bits() & LSB_MASK) as usize];

        // is not in ring buffer --> take previous
        if self.index < lsb_index || (self.index - lsb_index) >= 128 {
            lsb_index = self.index;
        }

        lsb_index %= 128;
        let ref_value = self.stored_vals[lsb_index];

        let xor = ref_value ^ value.to_bits();
        let trail = xor.trailing_zeros();
        let lead = xor.leading_zeros();

        let sig_bits = if xor == 0 { 1 } else { 64 - trail - lead };
        let sig_bytes = (sig_bits >> 3) + if sig_bits & 7 != 0 { 1 } else { 0 };

        let packed_metadata = (lsb_index as u32) << 9 | ((sig_bytes - 1) & 7) << 6 | (trail & 0x3f);
        self.w.write_bits(packed_metadata as u64, 16);

        if xor != 0 {
            self.w.write_bits(xor >> trail, sig_bytes * 8);
        }

        self.curr_idx += 1;
        self.curr_idx %= 128;

        self.stored_vals[self.curr_idx] = value.to_bits();

        self.index += 1;
        self.indices[(value.to_bits() & LSB_MASK) as usize] = self.index;

    }
}

impl Encode for Encoder {
    fn encode_vec(values: &Vec<f64>) -> Self {
        let mut patas = Encoder {
            first: true,
            stored_vals: vec![0; 128],
            indices: vec![0; 2_usize.pow(14)],
            curr_idx: 0,
            index: 0,
            w: OutputBitStream::with_capacity(values.len() / 2),
        };
        for &val in values {
            patas.encode(val);
        }
        patas
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
        this.w.write_bits(0xffff, 16);
        this.w.write_bit(0); // not sure why actual implementation does this
        let buffer = this.w.close();
        let len = &buffer.len() * 64;
        (buffer, len as u64)
    }
}

pub struct Decoder {
    first: bool,
    done: bool,

    stored_vals: Vec<u64>,
    curr: u64, // curr stored value
    curr_idx: usize,
    pub r: InputBitStream,
}

impl Decoder {
    pub fn new(r: InputBitStream) -> Self {
        Decoder {
            first: true,
            done: false,
            stored_vals: (0..128).collect(),
            curr: 0,
            curr_idx: 0,
            r,
        }
    }

    fn get_first(&mut self) -> Result<(), Error> {
        self.curr = self.r.read_bits(64)?;
        self.stored_vals[self.curr_idx] = self.curr;
        Ok(())
    }

    #[allow(unused)]
    fn get_value(&mut self) -> Result<(), Error> {
        let packed_metadata = self.r.read_bits(16)?;

        let lsb_index = packed_metadata as usize >> 9;
        let sig_bytes = ((packed_metadata as u32 >> 6) & 0b111) + 1;
        let trail = packed_metadata & 0x3f;

        if packed_metadata == 0xffff {
            self.curr = NAN;
            return Ok(());
        }

        if sig_bytes == 1 && trail == 0 {
            self.curr = self.stored_vals[lsb_index];
        } else {
            let ref_value = self.stored_vals[lsb_index];
            let xor = self.r.read_bits(sig_bytes * 8)? << trail;
            self.curr = ref_value ^ xor;
        }

        self.curr_idx += 1;
        self.curr_idx %= 128;
        self.stored_vals[self.curr_idx] = self.curr;

        Ok(())
    }

    fn get_next(&mut self) -> Result<u64, Error> {
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

impl Decode for Decoder {
    fn get_next(&mut self) -> Result<u64, Error> {
        self.get_next()
    }
}

#[cfg(test)]
mod test {
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

        // 0000000|110|000000 || 110101010101010101010101010101010101010101010101

        let (bytes, _) = encoder.close();
        let mut decoder = Decoder::new(InputBitStream::new(bytes));
        let mut datapoints = Vec::new();

        while let Ok(val) = decoder.get_next() {
            datapoints.push(f64::from_bits(val));
        }

        assert_eq!(datapoints, float_vec);
    }
}
