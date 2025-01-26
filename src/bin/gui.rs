use anyhow::Result;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, Color32};
use egui_plot::{Legend, Line, Plot, PlotPoints};

const RETENTION: usize = 3000;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 200.0]),
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

    let haystack: Vec<f32> = hound::WavReader::open("sounds/UndercityExample.wav")?
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();

    println!("target len: {}, haystack len: {}", target.len(), haystack.len());
    let capture_thread = thread::spawn(move || {
        for chunk in haystack.chunks(25) {
            chunk.iter().for_each(|value| {
                // pop one, push one, compute correlation (i.e. dot product of target and buffer)
                buffer.pop_front();
                buffer.push_back(*value);
            });
            // compute correlation
            let correlation = buffer
                .iter()
                .zip(target.iter())
                .map(|(a, b)| a * b)
                .sum::<f32>() / target_norm;
            {
                let mut correlations = correlations_clone.lock().unwrap();
                correlations.pop_front();
                correlations.push_back(correlation);
            }
            // sleep 0.5ms
            thread::sleep(std::time::Duration::from_micros(100));
        }
    });

    let correlations_clone2 = Arc::clone(&correlations);
    eframe::run_native(
        "frosk",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(correlations_clone2)))),
    )
    .unwrap();

    capture_thread.join().unwrap();

    Ok(())
}

struct MyApp {
    correlations: Arc<Mutex<VecDeque<f32>>>,
    time: u32,
}

impl MyApp {
    fn new(correlations: Arc<Mutex<VecDeque<f32>>>) -> Self {
        Self {
            correlations,
            time: 0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut plot_rect = None;
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
                .show_axes(egui::Vec2b::new(false, true))
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
                let inner = my_plot.show(ui, |plot_ui| {
                    let wave = Line::new(PlotPoints::from_ys_f32(&combined))
                        .color(Color32::from_rgb(200, 100, 100))
                        .style(egui_plot::LineStyle::Solid);
                    plot_ui.line(wave);
                });
                plot_rect = Some(inner.response.rect);
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