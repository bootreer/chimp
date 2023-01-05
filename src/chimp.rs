use crate::bitstream::{Error, InputBitStream, OutputBitStream};

const NAN: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    w: OutputBitStream,
}

// quick and dirty hack
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
            let lead = xor.leading_zeros();
            let trail = xor.trailing_zeros();

            if trail > 6 {
                self.w.write_bit(0);
                if xor == 0 {
                    self.w.write_bit(0);
                } else {
                    self.w.write_bit(1);
                    self.w.write_bits(lead as u64, 3);
                    let center_bits = 64 - lead - trail;
                    self.w.write_bits(center_bits as u64, 6);
                    self.w.write_bits(xor >> trail, center_bits);
                }
            } else {
                self.w.write_bit(1);
                if lead == self.leading_zeros {
                    self.w.write_bit(0);
                } else {
                    self.w.write_bit(1);
                    self.w.write_bits(lead as u64, 3);
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

        match tag {
            0 => Ok(self.curr),
            _ => unreachable!("bruh moment: somehow value not in [0,3] when reading 2 bits"),
        }
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
