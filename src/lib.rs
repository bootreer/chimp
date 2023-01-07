pub mod bitstream;
pub mod chimp;
pub mod gorilla;

static NAN: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;

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
}
