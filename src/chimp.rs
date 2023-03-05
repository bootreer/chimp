use crate::bitstream::*;
use crate::{Bit, Decode, Encode, LEADING_REPR_DEC, LEADING_REPR_ENC, LEADING_ROUND, NAN};

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
    size: u64,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            first: true,
            curr: 0,
            leading_zeros: u32::MAX,
            w: OutputBitStream::new(),
            size: 0,
        }
    }

    fn insert_first(&mut self, value: f64) {
        self.curr = value.to_bits();
        self.w.write_bits(self.curr, 64);
        self.size += 64;
    }

    fn insert_value(&mut self, value: f64) {
        let xor = self.curr ^ value.to_bits();

        if xor == 0 {
            self.w.write_bits(0, 2);
            self.size += 2;
            return;
        }

        let lead = LEADING_ROUND[xor.leading_zeros() as usize];
        let trail = xor.trailing_zeros();

        if trail > 6 {
            self.w.write_bits(1, 2);

            self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);
            let center_bits = 64 - lead - trail;
            if center_bits == 51 {
                println! {"hello: value: {}, curr: {}", value, self.curr};
            }
            self.w.write_bits(center_bits as u64, 6);
            self.w.write_bits(xor >> trail, center_bits);
            self.leading_zeros = 65;

            self.size += 11 + center_bits as u64;
        } else {
            self.w.write_bit(1);
            if lead == self.leading_zeros {
                self.w.write_bit(0);
            } else {
                self.leading_zeros = lead;
                self.w.write_bit(1);
                self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);

                self.size += 3;
            }
            self.w.write_bits(xor, 64 - lead);

            self.size += 66 - lead as u64;
        }
        self.curr = value.to_bits();
    }

    // TODO: impl this
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64")))]
    #[target_feature(enable = "avx2")] // TODO: enable AVX512
    #[allow(unused_variables)]
    unsafe fn simd_vec(&mut self, values: &Vec<f64>) {
        let v = values; // bruh moment
        self.insert_first(values[0]);

        let mut i = 0;

        //  we can in max. do xor on 4 values -> 4 new entries at a time
        //  windows() doesn't work as there would be too many overlapping xor-ed values -> increment i by 4 every iteration
        while i < values.len() - 4 {
            // mem::transmute maybe
            let (a, b, c, d, e) = (v[i], v[i + 1], v[i + 2], v[i + 3], v[i + 4]);
            // println!("a: {a}, b: {b}, c: {c}, d: {d}, e: {e}");

            // lol switch to mm512
            let xor = _mm256_castpd_si256(_mm256_xor_pd(
                _mm256_set_pd(a, b, c, d),
                _mm256_set_pd(b, c, d, e),
            ));
            let leading = _mm256_lzcnt_epi64(xor);

            // since there is no trailing zero simd intrinsic and we only need to check if bottom
            // 6 bits are set
            let trail_threshold = _mm256_and_si256(xor, _mm256_set1_epi64x(0x3f));

            // println!("xor:         {:?}", xor);
            // println!("leading:     {:?}", leading);
            // println!("trail thres: {:?}", trail_threshold);

            i += 4;
        }
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
            size: 0,
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
        (this.w.close(), this.size)
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
            49.4, 48.8, 46.4, 47.9, 48.7, 48.9, 48.8, 46.4, 47.9, 48.7, 48.9,
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
        }

        // assert_eq!(datapoints, float_vec);
    }
}
