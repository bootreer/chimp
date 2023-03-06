use crate::bitstream::*;
use crate::{Bit, Decode, Encode, LEADING_REPR_DEC, LEADING_REPR_ENC, LEADING_ROUND, NAN};
use crossbeam;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    w: OutputBitStream,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            first: true,
            curr: 0,
            leading_zeros: u32::MAX,
            w: OutputBitStream::new(),
        }
    }

    fn insert_first(&mut self, value: f64) {
        self.curr = value.to_bits();
        self.w.write_bits(self.curr, 64);
    }

    fn insert_value(&mut self, value: f64) {
        let xor = self.curr ^ value.to_bits();
        let trailing = xor & 0x3f;

        self.enc_aux(xor, trailing);

        self.curr = value.to_bits();
    }

    #[inline(always)]
    fn enc_aux(&mut self, xor: u64, trailing: u64) {
        if xor == 0 {
            self.w.write_bits(0, 2);
            return;
        }

        let lead = LEADING_ROUND[xor.leading_zeros() as usize];

        // we and-ed with 0b11_1111
        if trailing == 0 {
            let trail = xor.trailing_zeros();

            self.w.write_bits(1, 2);
            self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);

            let center_bits = 64 - lead - trail;

            self.w.write_bits(center_bits as u64, 6);
            self.w.write_bits(xor >> trail, center_bits);
            self.leading_zeros = 65;
        } else {
            self.w.write_bit(1);
            if lead == self.leading_zeros {
                self.w.write_bit(0);
            } else {
                self.leading_zeros = lead;
                self.w.write_bit(1);
                self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);
            }
            self.w.write_bits(xor, 64 - lead);
        }
    }

    // TODO: impl this
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64")))]
    #[target_feature(enable = "avx2")]
    pub unsafe fn simd_vec(&mut self, values: &Vec<f64>) {
        let v = values; // bruh moment
        self.insert_first(values[0]);

        let mut i = 0;

        // bitwise operations work can be done across a simd vector (xor, leading/trailing zeroes)
        //  windows() doesn't work as there would be too many overlapping xor-ed values -> increment i by 4 every iteration
        while i < values.len() - 4 {
            // mem::transmute maybe
            let (a, b, c, d, e) = (v[i], v[i + 1], v[i + 2], v[i + 3], v[i + 4]);

            // lol switch to mm512
            let xor_vec = _mm256_castpd_si256(_mm256_xor_pd(
                _mm256_set_pd(a, b, c, d),
                _mm256_set_pd(b, c, d, e),
            ));

            // these slow down performance
            // let xor_vec = _mm256_xor_epi64(
            //     _mm256_loadu_epi64(v[i..].as_ptr() as *const i64),
            //     _mm256_loadu_epi64(v[i+1..].as_ptr() as *const i64),
            // );
            // let leading_vec = _mm256_lzcnt_epi64(xor_vec);

            // since there is no trailing zero simd intrinsic and we only need to check if bottom
            // 6 bits are set
            let trail_threshold = _mm256_and_si256(xor_vec, _mm256_set1_epi64x(0x3f));

            let xor: (u64, u64, u64, u64) /* ,u64, u64, u64, u64) */ = std::mem::transmute(xor_vec);
            // let leading: (u64, u64, u64, u64) /* ,u64, u64, u64, u64) */ = std::mem::transmute(xor_vec);
            let trailing: (u64, u64, u64, u64) /* ,u64, u64, u64, u64) */ = std::mem::transmute(trail_threshold);

            // println!("xor:         {:?}", xor);
            // println!("xor_vec:     {:?}", xor_vec);
            // println!("leading:     {:?}", leading);
            // println!("trail thres: {:?}", trail_threshold);

            // self.enc_aux(xor.7, leading.7, trailing.7);
            // self.enc_aux(xor.6, leading.6, trailing.6);
            // self.enc_aux(xor.5, leading.5, trailing.5);
            // self.enc_aux(xor.4, leading.4, trailing.4);
            self.enc_aux(xor.3, trailing.3);
            self.enc_aux(xor.2, trailing.2);
            self.enc_aux(xor.1, trailing.1);
            self.enc_aux(xor.0, trailing.0);

            i += 4;
        }
        self.curr = v[i].to_bits();
        i += 1;

        // encode rest that don't fit in simd vector
        while i < values.len() {
            self.insert_value(values[i]);
            i += 1;
        }
    }

    #[allow(unused)]
    pub fn threaded(num_theads: u64, values: &Vec<f64>) -> Vec<Box<[u64]>> {
        vec![Box::new([0u64; 1])]
    }

    // NOTE: timestamps?
}

impl Encode for Encoder {
    fn encode_vec(values: &Vec<f64>) -> Self {
        // not much of a gain by guaranteeing a capacity
        let mut enc = Encoder {
            first: true,
            curr: 0,
            leading_zeros: u32::MAX,
            w: OutputBitStream::with_capacity(values.len()),
        };
        for &val in values {
            enc.encode(val);
        }
        enc
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
        let buffer = this.w.close();
        let len = &buffer.len() * 64;
        (buffer, len as u64)
    }
}

#[derive(Debug)]
pub struct Decoder {
    first: bool,
    done: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    r: InputBitStream,
}

impl Decoder {
    pub fn new(read: InputBitStream) -> Self {
        Decoder {
            first: true,
            done: false,
            curr: 0,
            leading_zeros: 0,
            trailing_zeros: 0,
            r: read,
        }
    }

    fn get_first(&mut self) -> Result<(), Error> {
        self.curr = self.r.read_bits(64)?;
        Ok(())
    }

    fn get_value(&mut self) -> Result<(), Error> {
        let mut center_bits: u32;
        let xor: u64;
        match self.r.read_bits(2)? {
            1 => {
                self.leading_zeros = LEADING_REPR_DEC[self.r.read_bits(3)? as usize];
                center_bits = self.r.read_bits(6)? as u32;
                if center_bits == 0 {
                    center_bits = 64;
                }
                self.trailing_zeros = 64 - center_bits - self.leading_zeros;
                xor = self.r.read_bits(center_bits)?;
                self.curr ^= xor << self.trailing_zeros;
            }
            2 => {
                center_bits = 64 - self.leading_zeros;
                xor = self.r.read_bits(center_bits)?;
                self.curr ^= xor;
            }
            3 => {
                self.leading_zeros = LEADING_REPR_DEC[self.r.read_bits(3)? as usize];
                center_bits = 64 - self.leading_zeros;
                xor = self.r.read_bits(center_bits)?;
                self.curr ^= xor;
            }
            _ => {} // unreachable!("bruh moment"),
        }
        Ok(())
    }

    // implement iterator?
    pub fn get_next(&mut self) -> Result<u64, Error> {
        if self.done {
            return Err(Error::EOF);
        }

        if self.first {
            self.first = false;
            self.get_first()?;
        } else {
            self.get_value()?;
        }

        if self.curr == NAN {
            self.done = true;
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
mod chimp_tests {
    use super::{Decoder, Encoder};
    use crate::bitstream::InputBitStream;
    use crate::Encode;

    #[test]
    fn simple_test() {
        let float_vec: Vec<f64> = [
            1.0, 1.0, 16.42, 1.0, 0.00123, 24435_f64, 0_f64, 420.69, 64.2, 49.4, 48.8, 46.4, 64.2,
            49.4, 48.8, 46.4, 47.9, 48.7, 48.9, 48.8, 46.4, 47.9, 48.7, 48.9, 123.0, 123.0,
            332232., 124642356., 1.1111111,
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

    // TODO:
    #[test]
    fn simd_test() {
        let float_vec: Vec<f64> = [
            1.0, 1.0, 16.42, 1.0, 0.00123, 24435_f64, 0_f64, 420.69, 64.2, 49.4, 48.8, 46.4, 64.2,
            49.4, 48.8, 46.4, 47.9, 48.7, 48.9, 48.8, 46.4, 47.9, 48.7, 48.9, 48.1, 48.12, 1., 2.,
            0.3,
        ]
        .to_vec();

        let mut encoder = Encoder::new();
        unsafe {
            encoder.simd_vec(&float_vec);
        }

        let (bytes, _) = encoder.close();
        let mut decoder = Decoder::new(InputBitStream::new(bytes));
        let mut datapoints = Vec::new();

        while let Ok(val) = decoder.get_next() {
            datapoints.push(f64::from_bits(val));
            // println!("value: {}", f64::from_bits(val));
        }

        assert_eq!(datapoints, float_vec);
    }
}
