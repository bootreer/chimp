use crate::bitstream::{Error, InputBitStream, OutputBitStream};
use crate::Bit;

const NAN: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;

#[derive(Debug)]
pub struct Encoder {
    first: bool,
    curr: u64, // current float value as bits
    leading_zeros: u32,
    trailing_zeros: u32,
    write: OutputBitStream,
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
                        .write_bits(xor >> self.trailing_zeros, center_bits);
                    //     .write_bits(value.to_bits() >> self.trailing_zeros, center_bits);
                } else {
                    self.write.write_bit(1);
                    self.write.write_bits(lead as u64, 6);
                    let center_bits = 64 - lead - trail;
                    self.write.write_bits((center_bits as u64) - 1, 6);
                    self.write.write_bits(xor >> trail, center_bits);

                    self.leading_zeros = lead;
                    self.trailing_zeros = trail;
                }
            }
        }
        self.curr = value.to_bits();
    }

    pub fn close(mut self) -> Box<[u8]> {
        self.insert_value(f64::NAN);
        self.write.write_bit(0);
        self.write.close()
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

    pub fn get_next(&mut self) -> Result<u64, Error> {
        if self.done {
            return Err(Error::EOF);
        }

        if self.first {
            self.first = false;
            self.curr = self.read.read_bits(64)?;
            if self.curr == NAN {
                self.done = true;
            }
            return Ok(self.curr);
        }

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

            if self.curr == NAN {
                self.done = true;
            }
        }
        return Ok(self.curr);
    }
}

#[cfg(test)]
mod tests {
    use super::{Decoder, Encoder};
    use crate::bitstream::InputBitStream;

    #[test]
    fn test() {
        let float_vec: Vec<f64> = [1.0, 16.42, 1.0, 0.00123, 24435_f64, 0_f64, 420.69].to_vec();

        let mut encoder = Encoder::new();

        for val in float_vec {
            encoder.insert_value(val);
        }

        let bytes = encoder.close();
        let mut decoder = Decoder::new(InputBitStream::new(bytes));
        let mut datapoints = Vec::new();

        loop {
            match decoder.get_next() {
                Ok(val) => {
                    datapoints.push(val);
                }
                Err(_) => {
                    println!("encountered problem");
                    break;
                }
            };
        }
        println!("size: {}", datapoints.len());
        for val in datapoints {
            println!("{}", f64::from_bits(val));
            println!("as bits {:064b}", val);
        }
    }
}
