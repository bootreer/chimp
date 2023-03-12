#![warn(rust_2018_idioms, rust_2021_compatibility, nonstandard_style)]
#![allow(unused_imports, dead_code)]
#![feature(stdsimd)]

use crate::bitstream::{Error, InputBitStream, OutputBitStream};
pub mod aligned;
pub mod bitstream;
pub mod chimp;
pub mod chimpn;
pub mod gorilla;

const NAN: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;

const LEADING_REPR_ENC: [u32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

const LEADING_REPR_DEC: [u32; 8] = [0, 8, 12, 16, 18, 20, 22, 24];

// rounded values so we on avg use less space while encoding length of leading zeros?
const LEADING_ROUND: [u32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 12, 12, 12, 12, 16, 16, 18, 18, 20, 20, 22, 22, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
];

// not entirely necessary tbh
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
    fn encode_vec(values: &Vec<f64>) -> Self;
    fn encode(&mut self, value: f64);

    /// returns Boxed Buffer and number of bits written
    fn close(self) -> (Box<[u8]>, u64);
}

pub trait Decode {
    fn get_next(&mut self) -> Result<u64, Error>;
}
