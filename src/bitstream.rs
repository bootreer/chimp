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

#[derive(Debug, Clone)]
pub struct OutputBitStream {
    buffer: Vec<u8>, // maybe store as string?
    pos: u8,         // position in curr byte
}

impl OutputBitStream {
    pub fn new() -> Self {
        OutputBitStream {
            buffer: Vec::new(),
            pos: 8, // cuz buff.len == 0
        }
    }

    fn check_grow(&mut self) {
        if self.pos == 8 {
            self.buffer.push(0); // increase size
            self.pos = 0;
        }
    }

    fn grow(&mut self) {
        self.buffer.push(0);
    }

    pub fn close(self) -> Box<[u8]> {
        self.buffer.into_boxed_slice()
    }

    pub fn write_bit(&mut self, bit: u8) {
        self.check_grow();
        let idx = self.buffer.len() - 1;
        self.buffer[idx] |= (bit & 1) << (7 - self.pos);
        self.pos += 1;
    }

    pub fn write_byte(&mut self, byte: u8) {
        if self.pos == 8 {
            self.grow();
            let idx = self.buffer.len() - 1;
            self.buffer[idx] = byte;
            return;
        }

        let idx = self.buffer.len() - 1;
        self.buffer[idx] |= byte >> self.pos;
        self.grow();
        self.buffer[idx + 1] |= byte.wrapping_shl(8 - self.pos as u32);
    }

    pub fn write_bits(&mut self, bits: u64, mut len: u32) {
        while len >= 8 {
            let to_write = bits >> (len - 8);
            self.write_byte(to_write as u8);
            len -= 8;
        }

        while len > 0 {
            let to_write = bits >> (len - 1);
            self.write_bit(to_write as u8);
            len -= 1;
        }
    }
}

#[derive(Debug)]
pub struct InputBitStream {
    buffer: Vec<u8>,
    pos: u8,      // where we are in curr byte
    index: usize, // where we are in buffer
}

impl InputBitStream {
    pub fn new(bytes: Box<[u8]>) -> Self {
        InputBitStream {
            buffer: bytes.into_vec(),
            pos: 0,
            index: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<Bit, Error> {
        if self.pos == 8 {
            self.index += 1;
            self.pos = 0;
        }
        let curr_byte = *self.buffer.get(self.index).ok_or(Error::EOF)?;
        self.pos += 1;

        if (curr_byte >> (8 - self.pos)) & 1 == 0 {
            Ok(Bit::Zero)
        } else {
            Ok(Bit::One)
        }
    }

    fn read_byte(&mut self) -> Result<u8, Error> {
        if self.pos == 8 {
            self.index += 1;
            return self.buffer.get(self.index).copied().ok_or(Error::EOF);
        }
        if self.pos == 0 {
            self.pos += 8;
            return self.buffer.get(self.index).copied().ok_or(Error::EOF);
        }

        let mut byte: u8 = 0;
        let mut curr_byte = *self.buffer.get(self.index).ok_or(Error::EOF)?;

        byte |= curr_byte.wrapping_shl(self.pos as u32);

        self.index += 1;
        curr_byte = *self.buffer.get(self.index).ok_or(Error::EOF)? >> (8 - self.pos); // schmexy

        byte |= curr_byte;

        Ok(byte)
    }

    pub fn read_bits(&mut self, mut len: u32) -> Result<u64, Error> {
        if len > 64 {
            len = 64;
        }

        let mut bits: u64 = 0;

        while len >= 8 {
            let byte = self.read_byte()? as u64;
            len -= 8;
            bits |= byte.wrapping_shl(len);
        }

        while len > 0 {
            let bit = self.read_bit()?.into_64();
            len -= 1;
            bits |= bit << len;
        }

        Ok(bits)
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

        b.write_bit(1);
        assert_eq!(b.buffer.len(), 2);
        assert_eq!(b.buffer[0], 0b0101_0101);
        assert_eq!(b.buffer[1], 0b1000_0000);
    }

    #[test]
    fn write_byte() {
        let mut b = OutputBitStream::new();

        b.write_byte(123);
        b.write_byte(42);
        b.write_byte(255);

        assert_eq!(b.buffer.len(), 3);
        assert_eq!(b.buffer[0], 123);
        assert_eq!(b.buffer[1], 42);
        assert_eq!(b.buffer[2], 255);

        b.write_bit(0);
        b.write_bit(0);
        b.write_bit(0);
        b.write_byte(0b1010_0101); // --> 0001_0100 ; 101x_xxxx
        b.write_byte(0b0000_1111); // --> 1010_0001 ; 111x_xxxx
        b.write_bit(0); // --> 1110_xxxx
        b.write_byte(0b1001_1111); // --> 1110_1001 ; 1111_0000

        assert_eq!(b.buffer.len(), 7);
        assert_eq!(b.buffer[3], 0b0001_0100);
        assert_eq!(b.buffer[4], 0b1010_0001);
        assert_eq!(b.buffer[5], 0b1110_1001);
        assert_eq!(b.buffer[6], 0b1111_0000);
    }

    #[test]
    fn write_bits() {
        let mut b = OutputBitStream::new();

        // 0001
        b.write_bits(1, 4);

        // 0 * 16
        b.write_bits(0, 16);

        // 11001
        b.write_bits(25, 5);

        // 1000101
        b.write_bits(69, 7);

        assert_eq!(b.buffer.len(), 4);
        assert_eq!(b.buffer[0], 0b0001_0000);
        assert_eq!(b.buffer[1], 0);
        assert_eq!(b.buffer[2], 0b0000_1100);
        assert_eq!(b.buffer[3], 0b1100_0101);
        assert_eq!(b.pos, 8);

        // 10110
        b.write_bits(0b1001_0110, 5);
        assert_eq!(b.buffer[4], 0b1011_0000);
    }

    #[test]
    fn write_read() {
        let mut b = OutputBitStream::new();
        b.write_bits(1.0_f64.to_bits(), 64);
        b.write_bits(0b1011, 4);

        let mut r = InputBitStream::new(b.close());
        assert_eq!(r.read_bits(64).unwrap(), 1.0_f64.to_bits());
        assert_eq!(r.read_bits(4).unwrap(), 0b1011);
    }
}
