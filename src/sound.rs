use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use rodio::{decoder::*, Sample, Source};

/// A streamed-in buffer of audio samples
#[derive(Debug, Clone)]
pub struct SoundBuffer<S> {
    samples: Arc<Mutex<Vec<S>>>,
    sample_rate: u32,
    channels: u16,
    done: Arc<AtomicBool>,
}

impl<S> SoundBuffer<S>
where
    S: Sample + Send + 'static,
{
    /// Get the sample rate of the sound
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    /// Get the number of channels the sound has
    pub fn channels(&self) -> u16 {
        self.channels
    }
    /// Load the sound from raw audio data
    pub fn from_raw(samples: Vec<S>, sample_rate: u32, channels: u16) -> Self {
        SoundBuffer {
            samples: Arc::new(Mutex::new(samples)),
            sample_rate,
            channels,
            done: Arc::new(AtomicBool::new(true)),
        }
    }
    /**
    Load the sound from encoded audio data

    The samples are streamed into the buffer as they are decoded

    Supports MP3, WAV, Vorbis and Flac
    */
    pub fn decode<T>(bytes: T) -> Result<Self, DecoderError>
    where
        T: AsRef<[u8]> + Send + 'static,
    {
        let decoder = Decoder::new(Cursor::new(bytes))?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done);
        thread::spawn(move || {
            for sample in decoder.convert_samples::<S>() {
                samples_clone.lock().unwrap().push(sample);
            }
            done_clone.store(true, Ordering::Relaxed);
        });
        Ok(SoundBuffer {
            samples,
            sample_rate,
            channels,
            done,
        })
    }
    /// Check if the sound has finished decoding
    pub fn finished_decoding(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }
}

/// A playable handle to a `SoundBuffer`
#[derive(Debug, Clone)]
pub struct SoundSource<S> {
    buffer: Arc<SoundBuffer<S>>,
    i: usize,
}

impl<S> From<Arc<SoundBuffer<S>>> for SoundSource<S> {
    fn from(buffer: Arc<SoundBuffer<S>>) -> Self {
        SoundSource { buffer, i: 0 }
    }
}

impl<S> Iterator for SoundSource<S>
where
    S: Sample,
{
    type Item = S;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.buffer.samples.lock().unwrap().get(self.i).copied() {
            self.i += 1;
            Some(sample)
        } else if Arc::strong_count(&self.buffer.samples) > 1 {
            Some(S::zero_value())
        } else {
            None
        }
    }
}

impl<S> Source for SoundSource<S>
where
    S: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.samples.lock().unwrap().len() - self.i)
    }
    fn channels(&self) -> u16 {
        self.buffer.channels
    }
    fn sample_rate(&self) -> u32 {
        self.buffer.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
