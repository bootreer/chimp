use crate::bitstream::{Error, InputBitStream, OutputBitStream};
use crate::{Bit, Encode, NAN};

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    write: OutputBitStream,
    size: u64,
}

// quick and dirty hack
impl Encoder {
    pub fn new() -> Self {
        Encoder {
            first: true,
            curr: 0,
            leading_zeros: u32::MAX,
            trailing_zeros: 0,
            write: OutputBitStream::new(),
            size: 0,
        }
    }

    pub fn insert_value(&mut self, value: f64) {
        if self.first {
            self.first = false;
            self.write.write_bits(value.to_bits(), 64);

            self.size += 64;
        } else {
            let xor = self.curr ^ value.to_bits();
            if xor == 0 {
                // identical
                self.write.write_bit(0);

                self.size += 1;
            } else {
                self.write.write_bit(1);
                let lead = xor.leading_zeros();
                let trail = xor.trailing_zeros();

                self.size += 2;
                if self.leading_zeros <= lead && self.trailing_zeros <= trail {
                    self.write.write_bit(0);
                    let center_bits = 64 - self.leading_zeros - self.trailing_zeros;

                    self.write
                        .write_bits(xor >> self.trailing_zeros, center_bits);
                    self.size += center_bits as u64;
                } else {
                    self.write.write_bit(1);
                    self.write.write_bits(lead as u64, 6);
                    let center_bits = 64 - lead - trail;
                    self.write.write_bits((center_bits as u64) - 1, 6);
                    self.write.write_bits(xor >> trail, center_bits);

                    self.leading_zeros = lead;
                    self.trailing_zeros = trail;

                    self.size += 12 + center_bits as u64;
                }
            }
        }
        self.curr = value.to_bits();
    }
    // TODO: timestamps?
}

impl Encode for Encoder {
    fn encode_vec(values: &Vec<f64>) -> Self {
        let mut enc = Encoder::new();
        for &val in values {
            enc.encode(val);
        }
        enc

    }

    fn encode(&mut self, value: f64) {
        self.insert_value(value);
    }

    fn close(self) -> (Box<[u8]>, u64) {
        let mut this = self;
        this.insert_value(f64::NAN);
        (this.write.close(), this.size)
    }
}

#[derive(Debug)]
pub struct Decoder {
    first: bool,
    done: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    read: InputBitStream,
}

impl Decoder {
    pub fn new(read: InputBitStream) -> Self {
        Decoder {
            first: true,
            done: false,
            curr: 0,
            leading_zeros: 0,
            trailing_zeros: 0,
            read,
        }
    }

    fn get_first(&mut self) -> Result<u64, Error> {
        self.curr = self.read.read_bits(64)?;
        Ok(self.curr)
    }

    fn get_value(&mut self) -> Result<u64, Error> {
        let mut bit = self.read.read_bit()?;
        if bit == Bit::One {
            bit = self.read.read_bit()?;
            if bit == Bit::One {
                self.leading_zeros = self.read.read_bits(6)? as u32;
                let center_bits = self.read.read_bits(6)? as u32 + 1;
                self.trailing_zeros = 64 - self.leading_zeros - center_bits;
            }

            let center_bits = 64 - self.leading_zeros - self.trailing_zeros;
            let xor = self.read.read_bits(center_bits)?;
            self.curr ^= xor << self.trailing_zeros;
        }
        Ok(self.curr)
    }

    pub fn get_next(&mut self) -> Result<u64, Error> {
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
            self.done = true;
            Err(Error::EOF)
        } else {
            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Decoder, Encoder};
    use crate::bitstream::InputBitStream;
    use crate::Encode;

    #[test]
    fn simple_test() {
        let float_vec: Vec<f64> = [
            1.0, 1.0, 16.42, 1.0, 0.00123, 24435_f64, 0_f64, 420.69, 64.2, 49.4, 48.8, 46.4,
        ]
        .to_vec();

        let mut encoder = Encoder::new();

        for val in &float_vec {
            encoder.insert_value(*val);
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
