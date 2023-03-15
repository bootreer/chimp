use chimp_lib::{aligned, bitstream::InputBitStream, chimp, chimpn, gorilla, Decode, Encode};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum ChimpType {
    Chimp,
    ChimpN,
    SIMD,
    Gorilla,
    Rayon,
    Patas,
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

    println!("-----------------CHIMP[RAYON]-----------------------");
    test_compression(&paths, ChimpType::Rayon);

    println!("-----------------PATAS------------------------------");
    test_compression(&paths, ChimpType::Patas);
}

pub fn test_compression(paths: &Vec<(&str, usize)>, enc_t: ChimpType) {
    let mut enc_vec = Vec::new();
    let mut dec_vec = Vec::new();
    let mut ratio_vec = Vec::new();

    // does n runs over each dataset, used for getting an avg dec/enc speed 
    let n = 25;
    for _ in 0..n {
        for (path, float_idx) in paths {
            // println!("[[DATASET: {}]]", path);
            let reader = csv::Reader::from_path(path);
            let mut values: Vec<f64> = Vec::new();

            for record in reader.unwrap().records() {
                let string_record = record.unwrap();
                let val = string_record[*float_idx].to_string();
                let val = val.parse::<f64>().unwrap();
                values.push(val);
            }
            let enc_speed: Duration;
            let dec_speed: Duration;
            let compr_ratio: f64;
            match enc_t {
                ChimpType::Rayon => {
                    let now = Instant::now();
                    let encoded = chimp::Encoder::threaded(&values);
                    let new_now = Instant::now();
                    // println!(
                    //     "[encode] per 1000 values: {:?}",
                    //     (new_now - now) / (values.len() / 1000) as u32
                    // );
                    let size: u64 = encoded.iter().fold(0, |acc, tup| acc + tup.1);
                    // println!(
                    //     "average bits per val: {}",
                    //     size as f64 / values.len() as f64
                    // );
                    enc_speed = (new_now - now) / (values.len() / 1000) as u32;
                    compr_ratio = size as f64 / values.len() as f64;

                    let now = Instant::now();
                    let decoded = chimp::Decoder::decode_threaded(encoded);
                    let new_now = Instant::now();

                    dec_speed = (new_now - now) / (decoded.len() / 1000) as u32;
                    // println!(
                    //     "[decode] per 1000 values: {:?}",
                    //     (new_now - now) / (decoded.len() / 1000) as u32
                    // );
                    assert_eq!(decoded, values);
                }
                ChimpType::SIMD => {
                    let mut chimp_simd = chimp::Encoder::new();
                    let now = Instant::now();
                    unsafe {
                        chimp_simd.simd_vec(&values);
                    }
                    let new_now = Instant::now();
                    let (buffer, size) = chimp_simd.close();
                    // println!(
                    //     "average bits per val: {}",
                    //     size as f64 / values.len() as f64
                    // );
                    // println!(
                    //     "[encode] per 1000 values: {:?}",
                    //     (new_now - now) / (values.len() / 1000) as u32
                    // );
                    enc_speed = (new_now - now) / (values.len() / 1000) as u32;
                    compr_ratio = size as f64 / values.len() as f64;

                    let mut dec = chimp::Decoder::new(InputBitStream::new(buffer));
                    let mut vec: Vec<f64> = Vec::new();
                    let now = Instant::now();
                    while let Ok(dec_val) = dec.get_next() {
                        vec.push(f64::from_bits(dec_val));
                    }
                    let new_now = Instant::now();

                    dec_speed = (new_now - now) / (vec.len() / 1000) as u32;
                    // println!(
                    //     "[decode] per 1000 values: {:?}",
                    //     (new_now - now) / (vec.len() / 1000) as u32
                    // );
                    assert_eq!(&vec, &values);
                }
                ChimpType::Chimp => {
                    (compr_ratio, enc_speed, dec_speed) = encode(
                        chimp::Encoder::with_capacity(values.len()),
                        &values,
                        ChimpType::Chimp,
                    );
                }
                ChimpType::ChimpN => {
                    (compr_ratio, enc_speed, dec_speed) = encode(
                        chimpn::Encoder::with_capacity(values.len()),
                        &values,
                        ChimpType::ChimpN,
                    );
                }
                ChimpType::Gorilla => {
                    (compr_ratio, enc_speed, dec_speed) =
                        encode(gorilla::Encoder::new(), &values, ChimpType::Gorilla);
                }
                ChimpType::Patas => {
                    (compr_ratio, enc_speed, dec_speed) = encode(
                        aligned::Encoder::with_capacity(values.len()),
                        &values,
                        ChimpType::Patas,
                    );
                }
            }
            enc_vec.push(enc_speed);
            dec_vec.push(dec_speed);
            ratio_vec.push(compr_ratio);
        }
    }
    let enc_avg: Duration =
        enc_vec.iter().fold(Duration::ZERO, |acc, dur| acc + *dur) / enc_vec.len() as u32;
    let dec_avg: Duration =
        dec_vec.iter().fold(Duration::ZERO, |acc, dur| acc + *dur) / dec_vec.len() as u32;
    let compr: f64 =
        ratio_vec.iter().fold(0f64, |acc, compr| acc + *compr) / ratio_vec.len() as f64;
    println!(
        "AVG Enc/1000: {:?} | AVG Dec/1000: {:?} | AVG Compr Ratio: {compr} bits/val",
        enc_avg, dec_avg
    );
}

// i've won but at what cost
// returns compression ratio, time/1000 for encoding and decoding
#[allow(unused_variables)]
pub fn encode(
    mut enc: impl Encode,
    values: &Vec<f64>,
    enc_t: ChimpType,
) -> (f64, Duration, Duration) {
    let now = Instant::now();
    for val in values {
        enc.encode(*val);
    }
    let new_now = Instant::now();
    let (bytes, size) = enc.close();
    /*
    println!(
        "average bits per val: {}",
        size as f64 / values.len() as f64
    );
    println!(
        "[encode] per 1000 values: {:?}",
        (new_now - now) / (values.len() / 1000) as u32
    );
    */
    let enc_speed = (new_now - now) / (values.len() / 1000) as u32;
    let compr_ratio = size as f64 / values.len() as f64;

    let bitstream = InputBitStream::new(bytes);

    // most readable return value
    (
        compr_ratio,
        enc_speed,
        match enc_t {
            ChimpType::Chimp => decode(chimp::Decoder::new(bitstream), values),
            ChimpType::ChimpN => decode(chimpn::Decoder::new(bitstream), values),
            ChimpType::Gorilla => decode(gorilla::Decoder::new(bitstream), values),
            ChimpType::Patas => decode(aligned::Decoder::new(bitstream), values),
            _ => Duration::ZERO,
        },
    )
}

#[allow(unused)]
pub fn decode(mut dec: impl Decode, values: &Vec<f64>) -> Duration {
    let mut vec: Vec<f64> = Vec::new();
    let now = Instant::now();

    while let Ok(dec_val) = dec.get_next() {
        vec.push(f64::from_bits(dec_val));
    }

    let new_now = Instant::now();
    // println!(
    //     "[decode] per 1000 values: {:?}",
    //     (new_now - now) / (vec.len() / 1000) as u32
    // );

    assert_eq!(&vec, values);

    (new_now - now) / (vec.len() / 1000) as u32
}
