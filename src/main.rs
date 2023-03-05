use chimp_lib::{aligned, bitstream::InputBitStream, chimp, chimpn, Decode, Encode};

use std::time::Instant;

#[derive(Debug)]
pub enum ChimpType {
    Chimp,
    ChimpN,
}

// simple benchmark/test/comparison with city_temperature dataset
fn main() {
    let reader = csv::Reader::from_path("datasets/city_temperature.csv");

    let mut values: Vec<f64> = Vec::new();

    for record in reader.unwrap().records() {
        let string_record = record.unwrap();
        let val = string_record[2].to_string();
        let val = val.parse::<f64>().unwrap();
        values.push(val);
    }

    encode(chimp::Encoder::new(), &values, ChimpType::Chimp);
    println!("----------------------------------------------------");
    encode(chimpn::Encoder::new(), &values, ChimpType::ChimpN);
    println!("----------------------------------------------------");

    let mut patas = aligned::Encoder::new();
    let now = Instant::now();
    for &val in &values {
        patas.insert(val);
    }
    let new_now = Instant::now();
    let (buffer, size) = patas.close();
    println!(
        "per 1000 values: {:?}",
        (new_now - now) / (values.len() / 1000) as u32
    );
    println!("{} bits per Value", size as f64 / values.len() as f64);

    let mut dec = aligned::Decoder::new(InputBitStream::new(buffer));
    let mut vec: Vec<f64> = Vec::new();
    let now = Instant::now();

    while let Ok(dec_val) = dec.get_next() {
        vec.push(f64::from_bits(dec_val));
    }

    let new_now = Instant::now();
    println!(
        "time required to decode {} values: {:?}",
        values.len(),
        new_now - now
    );
    println!(
        "per 1000 values: {:?}",
        (new_now - now) / (vec.len() / 1000) as u32
    );
    println!("size of vector: {}", vec.len());
    // assert_eq!(&vec, &values);
}

// i've won but at what cost
#[allow(unused_variables)]
pub fn encode(mut enc: impl Encode, values: &Vec<f64>, enc_t: ChimpType) {
    let now = Instant::now();
    for val in values {
        enc.encode(*val);
    }
    let new_now = Instant::now();
    let (bytes, size) = enc.close();
    println!(
        "[{:?}], avg bits per val: {}",
        enc_t,
        size as f64 / values.len() as f64
    );

    println!(
        "time required to encode {} values: {:?}",
        values.len(),
        new_now - now
    );
    println!(
        "per 1000 values: {:?}",
        (new_now - now) / (values.len() / 1000) as u32
    );

    let bitstream = InputBitStream::new(bytes);
    match enc_t {
        ChimpType::Chimp => decode(chimp::Decoder::new(bitstream), values),
        ChimpType::ChimpN => decode(chimpn::Decoder::new(bitstream), values),
    };
}

pub fn decode(mut dec: impl Decode, values: &Vec<f64>) {
    let mut vec: Vec<f64> = Vec::new();
    let now = Instant::now();

    while let Ok(dec_val) = dec.get_next() {
        vec.push(f64::from_bits(dec_val));
    }

    let new_now = Instant::now();
    println!(
        "time required to decode {} values: {:?}",
        values.len(),
        new_now - now
    );
    println!(
        "per 1000 values: {:?}",
        (new_now - now) / (vec.len() / 1000) as u32
    );
    assert_eq!(&vec, values);
}
