use anyhow::Result;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, Color32};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use frosk::core::{
    capture::{default_audio_capture, AudioCapture}, dsp::SignalProcessor, event::{handle_event, FroskEvent}
};


const RETENTION: usize = 8000;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 125.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_position((0.0, 350.0)),
        ..Default::default()
    };

    // let ringbuffer = StaticRb::<f32, TARGET_SAMPLE_COUNT>::default();
    // let (mut rb_prod, mut rb_cons) = ringbuffer.split();
    // for _ in 0..TARGET_SAMPLE_COUNT {
    //     rb_prod.try_push(0.0).unwrap();
    // }

    let mut signal_processor = SignalProcessor::default();

    let correlations: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::from(vec![0.0; RETENTION])));
    let correlations_clone = Arc::clone(&correlations);
    let events: Arc<Mutex<Vec<FroskEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_to_be_handled: Arc<Mutex<VecDeque<FroskEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));

    let events_clone = Arc::clone(&events);
    let events_to_be_handled_clone: Arc<Mutex<VecDeque<FroskEvent>>> =
        Arc::clone(&events_to_be_handled);
    let audio_capture = default_audio_capture();
    unsafe {
        audio_capture.capture_game_audio(move |chunk| {
            for small_chunk in chunk.chunks(10) {
                signal_processor.process_chunk(small_chunk);
                let correlation = signal_processor.compute_correlation();

                if let Some(event) = signal_processor.determine_event(correlation) {
                    events_clone.lock().unwrap().push(event);
                    events_to_be_handled_clone.lock().unwrap().push_back(event);
                }

                {
                    // put in a block here so the lock will be released immediately
                    let mut correlations = correlations_clone.lock().unwrap();
                    correlations.pop_front();
                    correlations.push_back(correlation);
                }
            }
        })?;
    }

    let events_to_be_handled_clone = Arc::clone(&events_to_be_handled);
    let event_handler_thread = thread::spawn(move || loop {
        let event = events_to_be_handled_clone.lock().unwrap().pop_front();
        if let Some(event) = event {
            handle_event(event).unwrap();
        }
        thread::sleep(std::time::Duration::from_millis(50));
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
