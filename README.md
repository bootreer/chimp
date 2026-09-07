# Chimp
[chimp compression](https://vldb.org/pvldb/vol15/p3058-liakos.pdf) in [rust](https://www.youtube.com/watch?v=dQw4w9WgXcQ). 

----------------------------

[chimp.rs](src/chimp.rs) and [chimpn.rs](src/chimpn.rs) implementations of compression from paper

[aligned.rs](src/aligned.rs) based off of [DuckDB's implementation of a byte-aligned variation of Chimp](https://github.com/duckdb/duckdb/pull/5044)

[gorilla.rs](src/gorilla.rs) compression as described in [this paper](https://www.vldb.org/pvldb/vol8/p1816-teller.pdf) and chimp paper

## my code
![shitsonfireyo](https://user-images.githubusercontent.com/72973431/211576509-1abf63b2-9340-4aad-908f-f6cda1ff9495.jpg)

*Maybe might still do:*
- make compression generic for f64 and f32
- idk if this will actually ever become a crate (small rewrite probably not a bad idea in that case lol)


## Benchmark Results

Evaluated on an Apple M2 Max machine across the included datasets (city_temperature.csv, Stocks-Germany-sample.txt, SSD_HDD_benchmarks.csv) with 25 runs per dataset:

| Codec | Compression Ratio | Encoding Latency (µs / 1000 vals) | Decoding Latency (µs / 1000 vals) |
| :--- | :--- | :--- | :--- |
| fastalp | 14.63 bits/val | 5.164 µs | 1.002 µs |
| Gorilla | 52.70 bits/val | 6.144 µs | 6.047 µs |
| Patas | 21.51 bits/val | 6.952 µs | 6.740 µs |
| Chimp128 | 17.29 bits/val | 7.609 µs | 7.534 µs |
| Chimp | 41.08 bits/val | 8.846 µs | 9.585 µs |
