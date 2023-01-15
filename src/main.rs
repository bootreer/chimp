use chimp::{bitstream::InputBitStream, Encode};
use std::error::Error;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    // simple benchmark/test with city_temperature dataset
    let reader = csv::Reader::from_path("datasets/city_temperature.csv");

    let mut values: Vec<f64> = Vec::new();
    let mut chimp = chimp::Encoder::new();

    for record in reader?.records() {
        let string_record = record?;
        let val = string_record[7].to_string();
        let val = val.parse::<f64>()?;
        values.push(val);
    }
    // CHIMP
    let now = Instant::now();
    for val in &values {
        chimp.encode(*val);
    }
    let new_now = Instant::now();
    println!(
        "[chimp] avg bits per val: {}",
        chimp.size as f64 / values.len() as f64
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

    let (bytes, _size) = chimp.close();
    let mut chimp = chimp::Decoder::new(InputBitStream::new(bytes));
    let mut chimp_vec: Vec<f64> = Vec::new();

    let now = Instant::now();
    while let Ok(val) = chimp.get_next() {
        chimp_vec.push(f64::from_bits(val));
    }

    let new_now = Instant::now();
    println!(
        "time required to decode {} values: {:?}",
        values.len(),
        new_now - now
    );
    assert_eq!(&chimp_vec, &values);

    Ok(())
}
