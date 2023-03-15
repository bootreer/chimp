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
