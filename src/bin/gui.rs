use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use enigo::{Direction::Click, Key, Keyboard};
use ringbuf::{traits::*, StaticRb};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, Color32};
use egui_plot::{Legend, Line, Plot, PlotPoints};

const RETENTION: usize = 8000;

const TARGET_BYTES: &[u8] = include_bytes!("../../sounds/FishBite.wav");
const fn target_sample_count() -> u32 {
    let data = TARGET_BYTES;
    let num_channels = u16::from_le_bytes([data[22], data[23]]);
    let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);
    let mut offset = 12;
    while offset + 8 < data.len() {
        let chunk_id = [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ];
        let chunk_size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        match chunk_id {
            [b'd', b'a', b't', b'a'] => {
                let data_chunk_size = chunk_size;
                let bytes_per_sample = (bits_per_sample / 8) as u32;
                return data_chunk_size / (num_channels as u32 * bytes_per_sample);
            }
            _ => {}
        }

        offset += 8 + chunk_size as usize; // Move to next chunk
    }

    return 0;
}

const TARGET_SAMPLE_COUNT: usize = target_sample_count() as usize;

fn handle_event(event: FroskEvent) -> Result<()> {
    match event {
        FroskEvent::FishBite { score: _ } => {
            let mut enigo = enigo::Enigo::new(&enigo::Settings::default())?;

            // reel it in
            enigo.key(Key::F9, Click)?;

            thread::sleep(std::time::Duration::from_millis(1000));

            // just spam this a few times for now
            for _ in 0..10 {
                enigo.key(Key::F10, Click)?;
                thread::sleep(std::time::Duration::from_millis(200));
            }

            Ok(())
        }
    }
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 125.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_position((0.0, 350.0)),
        ..Default::default()
    };

    let target: Vec<f32> = hound::WavReader::open("sounds/FishBite.wav")?
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();
    let target_norm: f32 = target.iter().map(|x| x.powi(2)).sum::<f32>();

    let ringbuffer = StaticRb::<f32, TARGET_SAMPLE_COUNT>::default();
    let (mut rb_prod, mut rb_cons) = ringbuffer.split();
    for _ in 0..TARGET_SAMPLE_COUNT {
        rb_prod.try_push(0.0).unwrap();
    }

    let correlations: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::from(vec![0.0; RETENTION])));

    let correlations_clone = Arc::clone(&correlations);
    let events: Arc<Mutex<Vec<FroskEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    let events_to_be_handled: Arc<Mutex<VecDeque<FroskEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));

    let host = cpal::default_host();
    let loopback_device = host
        .input_devices()?
        .find(|d| d.name().unwrap().contains("BlackHole 2ch"))
        .unwrap();
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Fixed(440),
    };

    let events_to_be_handled_clone: Arc<Mutex<VecDeque<FroskEvent>>> =
        Arc::clone(&events_to_be_handled);
    let stream = loopback_device.build_input_stream(
        &config,
        move |big_chunk: &[f32], _: &_| {
            for chunk in big_chunk.chunks(10) {
                rb_cons.skip(chunk.len());
                rb_prod.push_slice(chunk);

                let correlation = rb_cons
                    .iter()
                    .zip(target.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f32>()
                    / target_norm;

                if correlation > 0.3 {
                    let event = FroskEvent::FishBite { score: correlation };
                    events_clone.lock().unwrap().push(event);
                    events_to_be_handled_clone.lock().unwrap().push_back(event);
                }

                {
                    let mut correlations = correlations_clone.lock().unwrap();
                    correlations.pop_front();
                    correlations.push_back(correlation);
                }
            }
        },
        move |err| {
            eprintln!("an error occurred on stream: {}", err);
        },
        None,
    )?;

    stream.play()?;

    let events_to_be_handled_clone = Arc::clone(&events_to_be_handled);
    let event_handler_thread = thread::spawn(move || loop {
        let event = events_to_be_handled_clone.lock().unwrap().pop_front();
        if let Some(event) = event {
            handle_event(event).unwrap();
        }
    });

    eframe::run_native(
        "frosk",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MyApp::new(
                Arc::clone(&events),
                Arc::clone(&correlations),
            )))
        }),
    )
    .unwrap();

    event_handler_thread.join().unwrap();

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum FroskEvent {
    FishBite { score: f32 },
}

struct MyApp {
    events: Arc<Mutex<Vec<FroskEvent>>>,
    correlations: Arc<Mutex<VecDeque<f32>>>,
    time: u32,
}

impl MyApp {
    fn new(events: Arc<Mutex<Vec<FroskEvent>>>, correlations: Arc<Mutex<VecDeque<f32>>>) -> Self {
        Self {
            events,
            correlations,
            time: 0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("events")
            .exact_width(100.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Events");
                });

                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |scroll_ui| {
                        scroll_ui.spacing_mut().item_spacing.y = 4.0;
                        scroll_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                        {
                            let events = self.events.lock().unwrap();
                            events.iter().rev().for_each(|event| match event {
                                FroskEvent::FishBite { score } => {
                                    scroll_ui.label(format!("FishBite ({:.3})", score));
                                }
                            });
                        }
                    })
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.ctx().request_repaint();
            self.time += 1;

            let my_plot = Plot::new("Cross-correlation with target")
                .legend(Legend::default())
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .allow_boxed_zoom(false)
                .show_x(false)
                .show_y(false)
                .show_axes(egui::Vec2b::new(false, false))
                .auto_bounds(egui::Vec2b::new(true, true))
                .include_x(0.0)
                .include_x(RETENTION as f32)
                .include_y(-1.0)
                .include_y(1.0)
                .show_grid(false);

            {
                let correlations = self.correlations.lock().unwrap();
                let (slice1, slice2) = correlations.as_slices();
                let combined: Vec<f32> = slice1.iter().chain(slice2.iter()).cloned().collect();
                my_plot.show(ui, |plot_ui| {
                    let wave = Line::new(PlotPoints::from_ys_f32(&combined))
                        .color(Color32::from_rgb(200, 100, 100))
                        .style(egui_plot::LineStyle::Solid);
                    plot_ui.line(wave);
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deque_slice() {
        let mut c = VecDeque::from(vec![0.0; 20]);

        for i in 1..=30 {
            c.pop_front();
            c.push_back(i as f32);
        }

        let (slice1, slice2) = c.as_slices();
        let combined: Vec<f32> = slice1.iter().chain(slice2.iter()).cloned().collect();
        assert_eq!(combined.len(), 20);
        for i in 0..20 {
            assert_eq!(combined[i], (i + 11) as f32);
        }
    }

    #[test]
    fn test_rb_slice() {
        let rb = StaticRb::<f32, 20>::default();
        let (mut prod, mut cons) = rb.split();
        for _ in 0..20 {
            prod.try_push(0.0).unwrap();
        }

        let source: Vec<f32> = (1..=30).map(|x| x as f32).collect();
        for chunk in source.chunks(5) {
            cons.skip(chunk.len());
            prod.push_slice(chunk);
        }

        let (slice1, slice2) = cons.as_slices();
        let combined: Vec<f32> = slice1.iter().chain(slice2.iter()).cloned().collect();
        assert_eq!(combined.len(), 20);
        for i in 0..20 {
            assert_eq!(combined[i], (i + 11) as f32);
        }
    }
}
