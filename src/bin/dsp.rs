use rustfft::{FftPlanner, num_complex::Complex};
use rand::prelude::*;

use std::{collections::VecDeque, sync::{Arc, Mutex}};

fn main() {
    // let mut planner = FftPlanner::new();
    // let fft = planner.plan_fft_forward(440);
    let buffer = Arc::new(Mutex::new(VecDeque::from([Complex{ re: 0.2f32, im: 0.0f32 }; 440])));

    let thread_copy = buffer.clone();
    let capture_thread = std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        loop {
            let mut buffer = thread_copy.lock().unwrap();
            buffer.push_back(Complex{ re: rng.gen_range(-1.0..1.0), im: 0.0f32 });
            buffer.pop_front();
        }
    });

    // acquire lock and clone
    // let mut freqs: Vec<_> = buffer.lock().unwrap().iter().map(|x| *x).collect();
    // fft.process(&mut freqs);
    // println!("{:?}", freqs);

    let printer_thread = std::thread::spawn(move || {
        // prints the mean of the vec every second
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let buffer = buffer.lock().unwrap();
            let sum: f32 = buffer.iter().map(|x| x.re).sum();
            println!("Mean: {}", sum / buffer.len() as f32);
        }
    });

    capture_thread.join().unwrap();
    printer_thread.join().unwrap();
}