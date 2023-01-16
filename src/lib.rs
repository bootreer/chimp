#![warn(rust_2018_idioms, rust_2021_compatibility, nonstandard_style)]
#![allow(dead_code)]

use crate::bitstream::{Error, InputBitStream, OutputBitStream};
pub mod bitstream;
pub mod chimpn;
pub mod gorilla;

const NAN: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;

#[derive(PartialEq, PartialOrd)]
pub enum Bit {
    Zero,
    One,
}

impl Bit {
    pub fn into_64(&self) -> u64 {
        match self {
            Bit::Zero => 0,
            Bit::One => 1,
        }
    }
}

pub trait Encode {
    fn encode(&mut self, value: f64);
    fn close(mut self) -> (Box<[u8]>, u64);
}

// TODO: figure out better shit than 3 static arrays lmao
const LEADING_REPR_ENC: [u32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

const LEADING_REPR_DEC: [u32; 8] = [0, 8, 12, 16, 18, 20, 22, 24];

// rounded values so we on avg use less space while encoding lenght of leading zeros?
const LEADING_ROUND: [u32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 12, 12, 12, 12, 16, 16, 18, 18, 20, 20, 22, 22, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
];

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    w: OutputBitStream,
    pub size: u64,
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
            self.leading_zeros = 65;
            return;
        }

        let lead = LEADING_ROUND[xor.leading_zeros() as usize];
        let trail = xor.trailing_zeros();

        if trail > 6 {
            self.w.write_bits(1, 2);

            self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);
            let center_bits = 64 - lead - trail;
            self.w.write_bits(center_bits as u64, 6);
            self.w.write_bits(xor >> trail, center_bits);
            self.leading_zeros = 65;

            self.size += 9 + center_bits as u64;
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

            self.size += 64 - lead as u64;
        }
        self.curr = value.to_bits();

        self.size += 2;
    }

    // TODO: timestamps?
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

    fn close(mut self) -> (Box<[u8]>, u64) {
        self.insert_value(f64::NAN);
        self.w.write_bit(0); // not sure why actual implementation does this
        (self.w.close(), self.size) // TODO: wtf
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

#[cfg(test)]
mod chimp_tests {
    use super::{Decoder, Encoder};
    use crate::bitstream::InputBitStream;
    use crate::Encode;

    #[test]
    fn simple_test() {
        let float_vec: Vec<f64> = [
            // 1.0, 1.0, 16.42, 1.0, 0.00123, 24435_f64, 0_f64, 420.69, 64.2, 49.4, 48.8, 46.4,
            // 64.2, 49.4, 48.8, 46.4, 47.9, 48.7, 48.9,
            48.8, 46.4, 47.9, 48.7, 48.9,
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
