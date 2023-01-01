fn main() {
    let long: u32 = 0xDEADBEEF;
    let short: u8 = long as u8;
    println!("Hello, world!");
    println!("0x{:x}", short);
    println!("{:b}", f32::INFINITY.to_bits());
}
