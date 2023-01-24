use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn chimp_enc(c: &mut Criterion) {
    let reader = csv::Reader::from_path("datasets/city_temperature.csv");
    let mut values: Vec<f64> = Vec::new();

    for record in reader.unwrap().records() {
        let string_record = record.unwrap();
        let val = string_record[7].to_string();
        let val = val.parse::<f64>().unwrap();
        values.push(val);
    }

    c.bench_function("encode city temps", |b| {
        b.iter(|| {
            let _: chimp::Encoder = chimp::Encode::encode_vec(&values);
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(2));
    targets = chimp_enc
);

criterion_main!(benches);
