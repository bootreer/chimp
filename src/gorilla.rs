use crate::bitstream::{Error, InputBitStream, OutputBitStream};
use crate::Bit;

#[derive(Debug)]
pub struct Compressor {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    write: OutputBitStream,
}

// quick and dirty hack
impl Compressor {
    pub fn new() -> Self {
        Compressor {
            first: true,
            curr: 0,
            leading_zeros: u32::MAX,
            trailing_zeros: u32::MAX,
            write: OutputBitStream::new(),
        }
    }

    pub fn insert_value(&mut self, value: f64) {
        if self.first {
            self.first = false;
            self.write.write_bits(value.to_bits(), 64);
        } else {
            let xor = self.curr ^ value.to_bits();

            if xor == 0 {
                // identical
                self.write.write_bit(0);
            } else {
                self.write.write_bit(1);
                let lead = xor.leading_zeros();
                let trail = xor.trailing_zeros();

                if self.leading_zeros <= lead && self.trailing_zeros <= trail {
                    self.write.write_bit(0);
                    let center_bits = 64 - self.leading_zeros - self.trailing_zeros;

                    // facebook writes 'xor >> self.trailing_zeros'
                    self.write
                        .write_bits(value.to_bits() >> self.trailing_zeros, center_bits);
                } else {
                    self.write.write_bit(1);
                    self.write.write_bits(lead.into(), 5);
                    let center_bits = 64 - lead - trail;
                    self.write.write_bits(center_bits.into(), 6);
                    self.write.write_bits(xor >> trail, center_bits);

                    self.leading_zeros = lead;
                    self.trailing_zeros = trail;
                }
            }
        }
        self.curr = value.to_bits();
    }

    // TODO: timestamps?
}

#[derive(Debug)]
pub struct Decompressor {
    first: bool,
    _done: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    read: InputBitStream,
}

impl Decompressor {
    pub fn new(read: InputBitStream) -> Self {
        Decompressor {
            first: true,
            _done: false,
            curr: 0,
            leading_zeros: 0,
            trailing_zeros: 0,
            read,
        }
    }

    pub fn get_next(&mut self) -> Result<u64, Error> {
        if self.first {
            self.first = false;
            self.curr = self.read.read_bits(64)?;
            return Ok(self.curr);
        }

        let mut bit = self.read.read_bit()?;

        if bit == Bit::Zero {
            return Ok(self.curr);
        } else {
            bit = self.read.read_bit()?;
            if bit == Bit::One {
                self.leading_zeros = self.read.read_bits(5)? as u32;
                let center_bits = self.read.read_bits(6)? as u32 + 1;
                self.trailing_zeros = 64 - self.leading_zeros - center_bits;
            }

            let center_bits = 64 - self.leading_zeros - self.trailing_zeros;
            let xor = self.read.read_bits(center_bits)?;
            self.curr ^= xor.wrapping_shl(self.trailing_zeros);
            return Ok(self.curr);
        }
    }
}
