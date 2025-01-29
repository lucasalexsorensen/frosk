use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use eframe::egui::{self, Color32};
use egui_plot::{Legend, Line, Plot, PlotPoints};

const RETENTION: usize = 8000;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([650.0, 300.0]),
        ..Default::default()
    };

    let target: Vec<f32> = hound::WavReader::open("sounds/FishBite.wav")?
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();
    let target_norm: f32 = target.iter().map(|x| x.powi(2)).sum::<f32>();
    let mut buffer: VecDeque<f32> = VecDeque::from(vec![0.0; target.len()]);
    let correlations: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::from(vec![0.0; RETENTION])));

    
    let correlations_clone = Arc::clone(&correlations);
    let events: Arc<Mutex<Vec<FroskEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

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

    let stream = loopback_device.build_input_stream(
        &config,
        move |big_chunk: &[f32], _: &_| {
            for chunk in big_chunk.chunks(5) {
                chunk.iter().for_each(|value| {
                    // pop one, push one, compute correlation (i.e. dot product of target and buffer)
                    buffer.pop_front();
                    buffer.push_back(*value);
                });

                let correlation = buffer
                    .iter()
                    .zip(target.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f32>()
                    / target_norm;

                if correlation > 0.75 {
                    let mut events = events_clone.lock().unwrap();
                    events.push(FroskEvent::FishBite { score: correlation });
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

    let events_clone2 = Arc::clone(&events);
    let correlations_clone2 = Arc::clone(&correlations);

    eframe::run_native(
        "frosk",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(events_clone2, correlations_clone2)))),
    )
    .unwrap();

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum FroskEvent {
    FishBite { score: f32 }
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

        egui::SidePanel::left("events").exact_width(150.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Events");
            });

            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |scroll_ui| {
                scroll_ui.spacing_mut().item_spacing.y = 4.0;
                scroll_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                {
                    let events = self.events.lock().unwrap();
                    events.iter().rev().for_each(|event| {
                        match event {
                            FroskEvent::FishBite { score } => {
                                scroll_ui.label(format!("FishBite ({:.3})", score));
                            }
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

                // if lates correlation is above threshold, add event
                // if let Some(last) = correlations.back() {
                // if self.time % 100 == 0 {
                //     println!("FishBite detected at time {}", self.time);
                //     self.events.push(FroskEvent::FishBite { score: correlations.back().unwrap().clone() });
                // }
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
}
