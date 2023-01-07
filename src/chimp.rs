use crate::bitstream::{Error, InputBitStream, OutputBitStream};
use crate::NAN;

// could be map
static LEADING_REPR_ENC: [u32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

static LEADING_REPR_DEC: [u32; 8] = [0, 8, 12, 16, 18, 20, 22, 24];

// rounded values so we on avg use less space while encoding lenght of leading zeros?
static LEADING_ROUND: [u32; 64] = [
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

    pub fn insert_value(&mut self, value: f64) {
        if self.first {
            self.first = false;
            self.w.write_bits(value.to_bits(), 64);
        } else {
            let xor = self.curr ^ value.to_bits();
            let lead = LEADING_ROUND[xor.leading_zeros() as usize];
            let trail = xor.trailing_zeros();

            if trail > 6 {
                self.w.write_bit(0);
                if xor == 0 {
                    self.w.write_bit(0);
                } else {
                    self.w.write_bit(1);
                    self.w.write_bits(LEADING_REPR_ENC[lead as usize] as u64, 3);
                    let center_bits = 64 - lead - trail;
                    self.w.write_bits(center_bits as u64, 6);
                    self.w.write_bits(xor >> trail, center_bits);
                }
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
        self.curr = value.to_bits();
    }

    pub fn close(mut self) -> Box<[u8]> {
        self.insert_value(f64::NAN);
        self.w.write_bit(0);
        self.w.close()
    }

    // TODO: timestamps?
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

    fn get_first(&mut self) -> Result<u64, Error> {
        self.curr = self.r.read_bits(64)?;
        if self.curr == NAN {
            self.done = true;
        }
        Ok(self.curr)
    }

    fn get_value(&mut self) -> Result<u64, Error> {
        let tag = self.r.read_bits(2)?;

        let mut center_bits: u32;
        let xor: u64;
        match tag {
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
            _ => {} // unreachable!("bruh moment: somehow value not in [0,3] when reading 2 bits"),
        }
        return Ok(self.curr);
    }

    // implement iterator?
    pub fn next(&mut self) -> Result<u64, Error> {
        if self.done {
            return Err(Error::EOF);
        }

        let res: u64;
        if self.first {
            self.first = false;
            res = self.get_first()?;
        } else {
            res = self.get_value()?;
        }

        if res == NAN {
            Err(Error::EOF)
        } else {
            Ok(res)
        }
    }
}
