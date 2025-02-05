use anyhow::Result;
use enigo::{Direction::Click, Key, Keyboard};
use std::thread;

#[derive(Debug, Clone, Copy)]
pub enum FroskEvent {
    FishBite { score: f32 },
}

pub fn handle_event(event: FroskEvent) -> Result<()> {
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
