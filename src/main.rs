use chimp::bitstream::InputBitStream;
use chimp::{gorilla, Encode};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // simple benchmark/test with city_temperature dataset
    let reader = csv::Reader::from_path("datasets/city_temperature.csv");

    let mut values: Vec<f64> = Vec::new();
    let mut gorilla = gorilla::Encoder::new();
    let mut chimp = chimp::Encoder::new();

    for record in reader?.records() {
        let string_record = record?;
        let val = string_record[7].to_string();
        let val = val.parse::<f64>()?;
        values.push(val);
    }

    // GORILLA
    for val in &values {
        gorilla.encode(*val);
    }

    println!(
        "[gorilla] avg bits per val: {}",
        *&gorilla.size as f64 / values.len() as f64
    );

    let mut gorilla = gorilla::Decoder::new(InputBitStream::new(gorilla.close()));
    let mut gor_vec: Vec<f64> = Vec::new();

    loop {
        match gorilla.get_next() {
            Ok(val) => gor_vec.push(f64::from_bits(val)),
            _ => break,
        }
    }
    assert_eq!(&gor_vec, &values);

    // CHIMP
    for val in &values {
        chimp.encode(*val);
    }

    println!(
        "[chimp] avg bits per val: {}",
        *&chimp.size as f64 / values.len() as f64
    );

    let mut chimp = chimp::Decoder::new(InputBitStream::new(chimp.close()));
    let mut chimp_vec: Vec<f64> = Vec::new();

    loop {
        match chimp.get_next() {
            Ok(val) => chimp_vec.push(f64::from_bits(val)),
            _ => break,
        }
    }

    // shit's on fire yo
    assert_eq!(&chimp_vec, &values);

    Ok(())
}
