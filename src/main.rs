#[allow(unused_imports)]
use chimp_lib::{aligned, bitstream::InputBitStream, chimp, chimpn, Decode, Encode};

use std::time::Instant;

#[derive(Debug)]
pub enum ChimpType {
    Chimp,
    ChimpN,
    SIMD,
}

// simple benchmark/test/comparison with different datasets
#[allow(unused)]
fn main() {
    let paths =
        vec![
        ("datasets/city_temperature.csv", 2),
        ("datasets/SSD_HDD_benchmarks.csv", 2),
        ("datasets/Stocks-Germany-sample.txt", 2),

        // delete first 2-3 lines on the influxdb data
        ("datasets/influxdb2-sample-data/air-sensor-data/air-sensor-data-annotated.csv", 4),
        ("datasets/influxdb2-sample-data/bird-migration-data/bird-migration.csv", 6),
        ("datasets/influxdb2-sample-data/bitcoin-price-data/bitcoin-historical-annotated.csv", 4),
        // ("datasets/influxdb2-sample-data/noaa-ndbc-data/latest-observations.csv", 15),
    ];

    println!("-----------------CHIMP------------------------------");
    test_compression(&paths, ChimpType::Chimp);

    for size in vec![128u64] {
        println!(
            "-----------------CHIMP{:03}---------------------------",
            size
        );
        test_compression(&paths, ChimpType::ChimpN);
    }

    println!("-------------------SIMD-----------------------------");
    test_compression(&paths, ChimpType::SIMD);

    /*
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
                println!("-------------------SIMD-----------------------------");
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
        vec.len(),
        new_now - now
    );
    println!(
        "per 1000 values: {:?}",
        (new_now - now) / (vec.len() / 1000) as u32
    );
    assert_eq!(&vec, &values);
    */
}

pub fn test_compression(paths: &Vec<(&str, usize)>, enc_t: ChimpType) {
    for (path, float_idx) in paths {
        println!("DATASET: {}", path);
        let reader = csv::Reader::from_path(path);
        let mut values: Vec<f64> = Vec::new();

        for record in reader.unwrap().records() {
            let string_record = record.unwrap();
            let val = string_record[*float_idx].to_string();
            let val = val.parse::<f64>().unwrap();
            values.push(val);
        }

        match enc_t {
            ChimpType::SIMD => {
                let mut chimp_simd = chimp::Encoder::new();
                let now = Instant::now();
                unsafe {
                    chimp_simd.simd_vec(&values);
                }
                let new_now = Instant::now();
                let (buffer, size) = chimp_simd.close();
                println!(
                    "average bits per val: {}",
                    size as f64 / values.len() as f64
                );
                println!(
                    "[encode] per 1000 values: {:?}",
                    (new_now - now) / (values.len() / 1000) as u32
                );
                let mut dec = chimp::Decoder::new(InputBitStream::new(buffer));
                let mut vec: Vec<f64> = Vec::new();
                let now = Instant::now();
                while let Ok(dec_val) = dec.get_next() {
                    vec.push(f64::from_bits(dec_val));
                }
                let new_now = Instant::now();
                // println!(
                //     "time required to decode {} values: {:?}",
                //     vec.len(),
                //     new_now - now
                // );
                println!(
                    "[decode] per 1000 values: {:?}",
                    (new_now - now) / (vec.len() / 1000) as u32
                );
                assert_eq!(&vec, &values);
            }
            ChimpType::Chimp => {
                encode(chimp::Encoder::new(), &values, ChimpType::Chimp);
            }
            ChimpType::ChimpN => {
                encode(chimpn::Encoder::new(), &values, ChimpType::ChimpN);
            }
        }
    }
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
        "average bits per val: {}",
        size as f64 / values.len() as f64
    );

    // println!(
    //     "time required to encode {} values: {:?}",
    //     values.len(),
    //     new_now - now
    // );
    println!(
        "[encode] per 1000 values: {:?}",
        (new_now - now) / (values.len() / 1000) as u32
    );

    let bitstream = InputBitStream::new(bytes);
    match enc_t {
        ChimpType::Chimp | ChimpType::SIMD => decode(chimp::Decoder::new(bitstream), values),
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
    // println!(
    //     "time required to decode {} values: {:?}",
    //     vec.len(),
    //     new_now - now
    // );
    println!(
        "[decode] per 1000 values: {:?}",
        (new_now - now) / (vec.len() / 1000) as u32
    );
    assert_eq!(&vec, values);
}
