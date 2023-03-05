use crate::Bit;
use std::{error, fmt};

/// Error
///
/// Enum used to represent potential errors when interacting with a stream.
#[derive(Debug, PartialEq)]
pub enum Error {
    EOF,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::EOF => write!(f, "Encountered the end of the stream"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::EOF => "Encountered the end of the stream",
        }
    }
}

#[derive(Debug)]
pub struct OutputBitStream {
    pub buffer: Vec<u64>,
    pos: u32,   // position in curr byte; 0 is right-most bit
    curr: u64, // faster than constantly accessing buffer
}

impl OutputBitStream {
    pub fn new() -> Self {
        OutputBitStream {
            buffer: Vec::new(),
            pos: 0,
            curr: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        OutputBitStream {
            buffer: Vec::with_capacity(capacity),
            pos: 0,
            curr: 0,
        }
    }

    #[inline(always)]
    fn check_grow(&mut self) {
        if self.pos == 64 {
            self.buffer.push(self.curr); // increase size
            self.pos = 0;
            self.curr = 0;
        }
    }

    #[inline(always)]
    fn grow(&mut self) {
        self.buffer.push(self.curr);
        self.curr = 0;
    }

    pub fn close(mut self) -> Box<[u64]> {
        // println!("Buffer stats: Bits used: {}", (self.buffer.len() * 64) + self.pos as usize);
        if self.pos != 0 {
            self.buffer.push(self.curr);
        }
        self.buffer.into_boxed_slice() // FIX?
    }

    #[inline(always)]
    pub fn write_bit(&mut self, bit: u8) {
        self.check_grow();
        self.pos += 1;
        self.curr |= ((bit & 1) as u64) << (64 - self.pos);
    }

    // might be able to remove this entirely for u64
    #[inline(always)]
    pub fn write_byte(&mut self, byte: u8) {
        let byte: u64 = byte as u64;
        self.check_grow();

        if 64 - self.pos < 8 {
            self.curr |= (byte >> self.pos) as u64;
            self.grow();
            let byte: u128 = (byte as u128) << self.pos; // to avoid overflow
            self.curr |= byte as u64;
            self.pos = (self.pos + 8) & 63;
            return;
        }

        self.curr |= byte << (64 - self.pos);
        self.pos += 8;
    }

    // NOTE: maybe make len u8
    // len \in [0,64]
    #[inline(always)]
    pub fn write_bits(&mut self, mut bits: u64, mut len: u32) {
        // if len == 0 {
        //     return;
        // }

        self.check_grow();

        if 64 - self.pos < len {
            len -= (64 - self.pos) as u32;
            self.curr |= bits.overflowing_shr(len).0;
            self.grow();
            self.pos = 0;
        }
        bits <<= (64 - len) - self.pos;
        self.curr |= bits;
        self.pos += len;
    }
}

#[derive(Debug)]
pub struct InputBitStream {
    buffer: Vec<u64>,
    pos: u8,      // where we are in curr byte
    index: usize, // where we are in buffer
    curr: u64,
}

impl InputBitStream {
    pub fn new(buffer: Box<[u64]>) -> Self {
        let buffer = buffer.into_vec();
        let curr = *buffer.get(0).unwrap();

        InputBitStream {
            buffer,
            pos: 0,
            index: 0,
            curr,
        }
    }

    #[inline(always)]
    fn check_grow(&mut self) {
        if self.pos == 64 {
            self.index += 1;
            self.pos = 0;
            self.curr = *self.buffer.get(self.index).ok_or(Error::EOF).unwrap();
        }
    }

    #[inline(always)]
    pub fn read_bit(&mut self) -> Result<Bit, Error> {
        self.check_grow();
        self.pos += 1;

        if (self.curr >> (64 - self.pos)) & 1 == 0 {
            Ok(Bit::Zero)
        } else {
            Ok(Bit::One)
        }
    }

    // can probably remove as well
    #[inline(always)]
    fn read_byte(&mut self) -> Result<u8, Error> {
        self.check_grow();

        let mut byte: u8 = 0;
        if self.pos > 54 {
            byte |= (self.curr << (64 - self.pos)) as u8;
            self.index += 1;
            self.curr = *self.buffer.get(self.index).ok_or(Error::EOF)?;

            byte |= (self.curr >> self.pos) as u8;

            self.pos = (self.pos + 8) & 63;
            return Ok(byte);
        }

        let curr_stored = *self.buffer.get(self.index).ok_or(Error::EOF)?;

        byte = (curr_stored >> 54 - self.pos) as u8;

        Ok(byte)
    }

    #[inline(always)]
    #[allow(unused_mut, unused_variables)]
    pub fn read_bits(&mut self, mut len: u32) -> Result<u64, Error> {
        let mut bits: u64 = 0;
        let bit_mask: u64 = (1 << len - 1) | (1 << len - 1) - 1;

        self.check_grow();

        if (64 - self.pos) < len as u8 {
            len -= (64 - self.pos) as u32;
            bits |= self.curr << len;
            self.index += 1;
            self.curr = *self.buffer.get(self.index).ok_or(Error::EOF)?;
            self.pos = 0;
        }

        self.pos += len as u8;
        bits |= self.curr >> (64 - self.pos as u32);

        Ok(bits & bit_mask)
    }
}

#[cfg(test)]
mod tests {
    use super::InputBitStream;
    use super::OutputBitStream;
    #[test]
    fn write_bit() {
        let mut b = OutputBitStream::new();

        for i in 0..8 {
            // 0101_0101
            b.write_bit(i % 2);
        }
        b.grow();
        assert_eq!(b.buffer[0], 0b0101_0101 << 56);
    }

    #[test]
    fn write_byte() {}

    #[test]
    fn write_bits() {}

    #[test]
    fn write_and_close() {
        let mut b = OutputBitStream::new();
        for i in 0..8 {
            b.write_bit(i % 3); // 0100_1001
        }
        // 0001
        b.write_bits(1, 4);
        // 0 * 16
        b.write_bits(0, 16);
        // 11001
        b.write_bits(25, 5);
        // 1000101
        b.write_bits(69, 7);

        b.write_bit(1);

        b.write_bits(0b1000_1110, 8);
        b.write_bits(0b0100_1001, 8);
        b.write_bits(0b0000_0110, 8);
        b.write_bit(1);
        b.write_bits(0b101, 3);
        let slice = b.close();

        // 1001_0010|0001_0000|0000_0000|0000_1100|1100_0101|1100_0111|0010_0100|1000_0110
        // 0110_1000
        assert_eq!(slice.len(), 2);

        let mut r = InputBitStream::new(slice);
        assert_eq!(r.read_bits(4).unwrap(), 0b0100);
        assert_eq!(r.read_bits(1).unwrap(), 0b1);
        assert_eq!(r.read_bits(1).unwrap(), 0b0);
        assert_eq!(r.read_bits(2).unwrap(), 0b01);

        assert_eq!(r.read_bits(4).unwrap(), 1);
        assert_eq!(r.read_bits(21).unwrap(), 0b11001);
    }

    #[test]
    fn write_read() {
        let mut b = OutputBitStream::new();
        b.write_bits(1.0_f64.to_bits(), 64);
        b.write_bits(0b1011, 4);

        let mut r = InputBitStream::new(b.close());
        assert_eq!(r.read_bits(64).unwrap(), 1.0_f64.to_bits());
        assert_eq!(r.read_bits(4).unwrap(), 0b1011);
        assert_eq!(r.read_bits(60).unwrap(), 0);
    }
}
