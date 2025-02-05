use std::{mem::MaybeUninit, sync::Arc};

use ringbuf::{storage::Owning, traits::*, wrap::caching::Caching, SharedRb, StaticRb};

use super::event::FroskEvent;

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
        if let [b'd', b'a', b't', b'a'] = chunk_id {
            let data_chunk_size = chunk_size;
            let bytes_per_sample = (bits_per_sample / 8) as u32;
            return data_chunk_size / (num_channels as u32 * bytes_per_sample);
        }

        offset += 8 + chunk_size as usize; // Move to next chunk
    }

    0
}

const TARGET_SAMPLE_COUNT: usize = target_sample_count() as usize;

#[derive(Default)]
pub struct SignalProcessor {
    buffer: Buffer,
    target: Target,
}

impl SignalProcessor {
    pub fn process_chunk(&mut self, chunk: &[f32]) {
        self.buffer.process_chunk(chunk);
    }

    pub fn determine_event(correlation: f32) -> Option<FroskEvent> {
        if correlation > 0.3 {
            return Some(FroskEvent::FishBite { score: correlation });
        }
        None
    }

    pub fn compute_correlation(&self) -> f32 {
        self.buffer
            .rb_cons
            .iter()
            .zip(self.target.target.iter())
            .map(|(a, b)| a * b)
            .sum::<f32>()
            / self.target.norm
    }
}

// TODO: find a nicer way to express this
type RbProdType =
    Caching<Arc<SharedRb<Owning<[MaybeUninit<f32>; TARGET_SAMPLE_COUNT]>>>, true, false>;
type RbConsType =
    Caching<Arc<SharedRb<Owning<[MaybeUninit<f32>; TARGET_SAMPLE_COUNT]>>>, false, true>;
struct Buffer {
    rb_prod: RbProdType,
    rb_cons: RbConsType,
}

impl Default for Buffer {
    fn default() -> Self {
        let ringbuffer = StaticRb::<f32, TARGET_SAMPLE_COUNT>::default();
        let (mut rb_prod, rb_cons) = ringbuffer.split();
        for _ in 0..TARGET_SAMPLE_COUNT {
            rb_prod.try_push(0.0).unwrap();
        }
        Self { rb_prod, rb_cons }
    }
}

impl Buffer {
    pub fn process_chunk(&mut self, chunk: &[f32]) {
        self.rb_cons.skip(chunk.len());
        self.rb_prod.push_slice(chunk);
    }
}

struct Target {
    target: Vec<f32>,
    norm: f32,
}

impl Default for Target {
    fn default() -> Self {
        let target: Vec<f32> = hound::WavReader::open("sounds/FishBite.wav")
            .unwrap()
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / i32::MAX as f32)
            .collect();
        let norm = target.iter().map(|x| x.powi(2)).sum::<f32>();
        Self { target, norm }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

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
