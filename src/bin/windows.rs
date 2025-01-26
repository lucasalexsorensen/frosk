// use frosk::core::process::windows::get_window_info;
// use frosk::core::capture::windows::capture_audio_for_process;
// use rustfft::{FftPlanner, num_complex::Complex};
// use anyhow::Result;
// use hound::WavReader;


// fn main() -> Result<()> {
//     let window = get_window_info("World of Warcraft")?;

//     let mut reader = WavReader::open("sounds/FishBite.wav")?;
//     let mut target_spectrum: Vec<_> = reader.samples::<i32>().map(|s| Complex { re: s.unwrap(), im: 0 }).collect();

//     let mut planner = FftPlanner::<i32>::new();
//     let fft = planner.plan_fft_forward(target_spectrum.len());
//     fft.process(&mut target_spectrum);

//     // given N (), we then want to maintan a buffer of the last N samples (plus some overhead), where N is the number of samples in the target spectrum

//     println!("Starting capture thread...");
//     // let capture_thread = std::thread::spawn(move || {
//     //     unsafe {
//     //         capture_audio_for_process(window.process_id, |data| {
//     //             println!("mean of samples: {:?}", data.iter().sum::<i32>() as f32 / data.len() as f32);
//     //         }).unwrap();
//     //     }
//     // });

//     // capture_thread.join().unwrap();

//     Ok(())
// }

fn main() {
    println!("i should do some cfg(windows) stuff here");
}