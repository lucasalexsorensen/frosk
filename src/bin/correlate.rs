use std::{sync::{Arc, Mutex}, thread};
use frosk::core::capture::{macos::MacOsCapturer, AudioCapture};
use anyhow::Result;
use rand::Rng;
use std::collections::VecDeque;

fn main () -> Result<()> {
    let target: Vec<i32> = hound::WavReader::open("sounds/FishBite.wav")?.samples().map(|s| s.unwrap()).collect();

    let target_norm: f32 = target.iter().map(|x| (*x as f32).powi(2)).sum::<f32>().sqrt();

    // sample 1 million random numbers
    let mut rng = rand::thread_rng();
    let mut source = Vec::with_capacity(1_000_000);
    for _ in 0..1_000_000 {
        source.push(rng.gen::<i32>());
    }

    // maintain a buffer of the most recent N received samples (where N is the length of the target
    let mut buffer: VecDeque<i32> = VecDeque::from(vec![0; target.len()]);
    
    let correlations: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

    let correlations_clone = Arc::clone(&correlations);
    // pretend we are sampling 440 samples at a time (roughly every 10 ms)
    let capture_thread = thread::spawn(move || {
        for chunk in source.chunks(440) {
            // process chunk one value at a time
            chunk.iter().for_each(|value| {
                // pop one, push one, compute correlation (i.e. dot product of target and buffer)
                buffer.pop_front();
                buffer.push_back(*value);
            });
            // compute correlation
            let correlation = buffer.iter().zip(target.iter()).map(|(a, b)| a * b).sum();
            {
                let mut correlations = correlations_clone.lock().unwrap();
                correlations.push(correlation);
            }

            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    let printer_thread = thread::spawn(move || {
        loop {
            {
                let correlations = correlations.lock().unwrap();
                println!("{:?}", correlations.len());
            }
            thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    capture_thread.join().unwrap();
    printer_thread.join().unwrap();

    Ok(())
}