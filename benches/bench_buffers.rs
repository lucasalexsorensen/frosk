use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;
use std::collections::VecDeque;
use ringbuf::{traits::*, StaticRb, HeapRb};


const N: usize = 1_000_000;
const B: usize = 100_000;
const C: usize = 25;

pub fn criterion_benchmark(c: &mut Criterion) {
    // how fast can we process a 1 million random elements, 440 elements at a time
    // buffer size should be 100k
    let source = (0..N).map(|_| random::<f32>()).collect::<Vec<f32>>();
    let source_slice = source.as_slice();

    let mut buffer = VecDeque::from(vec![0.0; B]);
    c.bench_function("VecDeque", |b| b.iter(|| {
        source_slice.chunks(C).for_each(|chunk| {
            chunk.iter().for_each(|value| {
                buffer.pop_front();
                buffer.push_back(*value);
            });
        });
    }));

    let mut buffer = StaticRb::<f32, B>::default();
    let (mut prod, mut cons) = buffer.split_ref();
    c.bench_function("StaticRb", |b| b.iter(|| {
        source_slice.chunks(C).for_each(|chunk| {
            cons.skip(chunk.len());
            prod.push_slice(chunk);
        });
    }));

    let mut buffer = HeapRb::<f32>::new(B);
    let (mut prod, mut cons) = buffer.split_ref();
    // fill it with zeros
    for _ in 0..B {
        prod.try_push(0.0).unwrap();
    }

    c.bench_function("HeapRb", |b| b.iter(|| {
        source_slice.chunks(C).for_each(|chunk| {
            cons.skip(chunk.len());
            prod.push_slice(chunk);
        });
    }));

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);