use crate::*;
use crate::chimpn::{THRESHOLD, LSB_MASK};
// since chimp compression doesn't guarantee byte alignedness, 
// added this to have decode and encode perform better

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    w: OutputBitStream,
    size: u64,

    stored_vals: Vec<u64>, // since Chimp128 offers close to 50% compression
    indices: Vec<usize>,
}

// TODO: conception/implementation lol
