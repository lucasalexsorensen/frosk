use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

use cpal::{Data, FromSample, Sample};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    StreamConfig,
};

// fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
//     hound::WavSpec {
//         channels: config.channels() as _,
//         sample_rate: config.sample_rate().0 as _,
//         bits_per_sample: (config.sample_format().sample_size() * 8) as _,
//         sample_format: match config.sample_format().is_float() {
//             true => hound::SampleFormat::Float,
//             false => hound::SampleFormat::Int,
//         }
//     }
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();

    let stereo_mix_device = host
        .input_devices()?
        .find(|d| d.name().unwrap().contains("CABLE Output"))
        .unwrap();
    let config = stereo_mix_device.default_input_config().expect("no default input config!");


    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    // let spec = wav_spec_from_config(&config);
    // let writer = hound::WavWriter::create(PATH, spec)?;
    // let writer = Arc::new(Mutex::new(Some(writer)));

    //let writer_2 = writer.clone();

    let stream = stereo_mix_device.build_input_stream(
        &config.into(), 
        move |data: &[f32], _: &_| {
            // write_input_data::<f32, f32>(data, &writer_2)
        },
        move |err| {
            eprintln!("an error occurred on stream: {}", err);
        },
        None
    )?;


    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    drop(stream);

    Ok(())
}


// type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

// fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
// where
//     T: Sample,
//     U: Sample + hound::Sample + FromSample<T>,
// {
//     if let Ok(mut guard) = writer.try_lock() {
//         if let Some(writer) = guard.as_mut() {
//             for &sample in input.iter() {
//                 let sample: U = U::from_sample(sample);
//                 writer.write_sample(sample).ok();
//             }
//         }
//     }
// }