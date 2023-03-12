use chimp_lib::{aligned, bitstream::InputBitStream, chimp, chimpn, gorilla, Decode, Encode};

use std::time::Instant;

#[derive(Debug)]
pub enum ChimpType {
    Chimp,
    ChimpN,
    SIMD,
    Gorilla,
    Rayon,
    Patas
}

// simple benchmark/test/comparison with different datasets
#[allow(unused)]
fn main() {
    let paths = vec![
        // ("datasets/autocorrelated_values.csv", 0),
        // ("datasets/random_values.csv", 0),

        ("datasets/city_temperature.csv", 2),
        ("datasets/Stocks-Germany-sample.txt", 2),
        ("datasets/SSD_HDD_benchmarks.csv", 2),

        // delete first 2-3 lines on the influxdb data
        // ("datasets/influxdb2-sample-data/air-sensor-data/air-sensor-data-annotated.csv", 4),
        // ("datasets/influxdb2-sample-data/bird-migration-data/bird-migration.csv", 6),
        // ("datasets/influxdb2-sample-data/bitcoin-price-data/bitcoin-historical-annotated.csv", 4),
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

    println!("-----------------GORILLA----------------------------");
    test_compression(&paths, ChimpType::Gorilla);

    println!("-------------------SIMD-----------------------------");
    test_compression(&paths, ChimpType::SIMD);

    // println!("-----------------CHIMP[RAYON]-----------------------");
    // test_compression(&paths, ChimpType::Rayon);

    println!("-----------------PATAS------------------------------");
    test_compression(&paths, ChimpType::Patas);
}

pub fn test_compression(paths: &Vec<(&str, usize)>, enc_t: ChimpType) {
    for (path, float_idx) in paths {
        println!("[[DATASET: {}]]", path);
        let reader = csv::Reader::from_path(path);
        let mut values: Vec<f64> = Vec::new();

        for record in reader.unwrap().records() {
            let string_record = record.unwrap();
            let val = string_record[*float_idx].to_string();
            let val = val.parse::<f64>().unwrap();
            values.push(val);
        }

        match enc_t {
            ChimpType::Rayon => {
                let now = Instant::now();
                let encoded = chimp::Encoder::threaded(&values);
                let new_now = Instant::now();
                println!(
                    "[encode] per 1000 values: {:?}",
                    (new_now - now) / (values.len() / 1000) as u32
                );
                let size: u64 = encoded.iter().fold(0, |acc, tup| acc + tup.1);
                println!(
                    "average bits per val: {}",
                    size as f64 / values.len() as f64
                );
                let now = Instant::now();
                let decoded = chimp::Decoder::decode_threaded(encoded);
                let new_now = Instant::now();
                println!(
                    "[decode] per 1000 values: {:?}",
                    (new_now - now) / (decoded.len() / 1000) as u32
                );
                assert_eq!(decoded, values);
            },
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
                println!(
                    "[decode] per 1000 values: {:?}",
                    (new_now - now) / (vec.len() / 1000) as u32
                );
                assert_eq!(&vec, &values);
            }
            ChimpType::Chimp => {
                encode(chimp::Encoder::with_capacity(values.len()), &values, ChimpType::Chimp);
            }
            ChimpType::ChimpN => {
                encode(chimpn::Encoder::with_capacity(values.len()), &values, ChimpType::ChimpN);
            }
            ChimpType::Gorilla => {
                encode(gorilla::Encoder::new(), &values, ChimpType::Gorilla);
            },
            ChimpType::Patas => {
                encode(aligned::Encoder::with_capacity(values.len()), &values, ChimpType::Patas);
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
    println!(
        "[encode] per 1000 values: {:?}",
        (new_now - now) / (values.len() / 1000) as u32
    );

    let bitstream = InputBitStream::new(bytes);
    match enc_t {
        ChimpType::Chimp => decode(chimp::Decoder::new(bitstream), values),
        ChimpType::ChimpN => decode(chimpn::Decoder::new(bitstream), values),
        ChimpType::Gorilla => decode(gorilla::Decoder::new(bitstream), values),
        ChimpType::Patas => decode(aligned::Decoder::new(bitstream), values),
        _ => {},
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
        "[decode] per 1000 values: {:?}",
        (new_now - now) / (vec.len() / 1000) as u32
    );
    assert_eq!(&vec, values);
}
