use std::{
    collections::HashMap,
    hash::Hash,
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crossbeam_utils::atomic::AtomicCell;
use rodio::{decoder::*, dynamic_mixer::*, source::Zero, Sample, Sink};

pub use rodio::{self, Source};

pub(crate) fn sink() -> Sink {
    let (send, recv) = mpsc::channel();
    thread::spawn(move || {
        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);
        send.send(sink).unwrap();
        loop {
            thread::sleep(Duration::from_secs(100));
        }
    });
    recv.recv().unwrap()
}

/// A master audio mixer
pub struct Mixer {
    mixer: Arc<DynamicMixerController<f32>>,
    volume: VolumeControl,
}

impl Mixer {
    pub(crate) fn new(sink: &Sink) -> Self {
        let (mixer, mixer_source) = mixer::<f32>(2, 44100);
        mixer.add(Zero::new(2, 44100));
        let volume = VolumeControl::default();
        let controlled_mixer = volume.control(mixer_source);
        sink.append(controlled_mixer);
        Mixer { mixer, volume }
    }
    /// Get a reference to the volume controller
    pub fn volume(&self) -> &VolumeControl {
        &self.volume
    }
    /// Play a sound source
    pub fn play<S>(&self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
    {
        self.mixer.add(source.convert_samples());
    }
}

/// An audio buffer cache
pub struct Sounds<S = ()>(HashMap<S, Arc<SoundBuffer>>);

impl<S> Default for Sounds<S> {
    fn default() -> Self {
        Sounds(HashMap::default())
    }
}

impl<S> Sounds<S>
where
    S: Eq + Hash,
{
    /// Insert a sound buffer
    pub fn insert(&mut self, sound_id: S, buffer: SoundBuffer) {
        self.0.insert(sound_id, Arc::new(buffer));
    }
    /// Check if a buffer exists for the given sound id
    pub fn contains(&self, sound_id: S) -> bool {
        self.0.contains_key(&sound_id)
    }
    /// Get a sound based on its id
    pub fn get(&self, sound_id: S) -> Option<&Arc<SoundBuffer>> {
        self.0.get(&sound_id)
    }
    /// Clear all sounds
    pub fn clear(&mut self) {
        self.0.clear()
    }
    /// Remove a sound
    pub fn remove(&mut self, sound_id: S) {
        self.0.remove(&sound_id);
    }
}

/// A streamed-in buffer of audio samples
#[derive(Debug, Clone)]
pub struct SoundBuffer {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    done: Arc<AtomicBool>,
}

impl SoundBuffer {
    /// Get the sample rate of the sound
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    /// Get the number of channels the sound has
    pub fn channels(&self) -> u16 {
        self.channels
    }
    /// Load the sound from raw audio data
    pub fn from_raw(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
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
            for sample in decoder.convert_samples::<f32>() {
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
pub struct SoundSource {
    buffer: Arc<SoundBuffer>,
    i: usize,
}

impl From<Arc<SoundBuffer>> for SoundSource {
    fn from(buffer: Arc<SoundBuffer>) -> Self {
        SoundSource { buffer, i: 0 }
    }
}

impl Iterator for SoundSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.buffer.samples.lock().unwrap().get(self.i).copied() {
            self.i += 1;
            Some(sample)
        } else if Arc::strong_count(&self.buffer.samples) > 1 {
            Some(0.0)
        } else {
            None
        }
    }
}

impl Source for SoundSource {
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

/// A volume controller
#[derive(Debug, Clone)]
pub struct VolumeControl {
    mute: Arc<AtomicCell<bool>>,
    volume: Arc<AtomicCell<f32>>,
}

impl Default for VolumeControl {
    fn default() -> Self {
        VolumeControl {
            mute: Arc::new(AtomicCell::new(false)),
            volume: Arc::new(AtomicCell::new(1.0)),
        }
    }
}

impl VolumeControl {
    /// Use this volume to control a source
    pub(crate) fn control<S>(&self, source: S) -> VolumeControlSource<S> {
        VolumeControlSource {
            source,
            mute: self.mute.clone(),
            volume: self.volume.clone(),
        }
    }
    /// Get the mute state
    pub fn mute(&self) -> bool {
        self.mute.load()
    }
    /// Set the m ute state
    pub fn set_mute(&self, mute: bool) {
        self.mute.store(mute);
    }
    /// Get the volume
    pub fn volume(&self) -> f32 {
        self.volume.load()
    }
    /// Set the volume
    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume);
    }
}

pub(crate) struct VolumeControlSource<T> {
    source: T,
    mute: Arc<AtomicCell<bool>>,
    volume: Arc<AtomicCell<f32>>,
}

impl<T> Iterator for VolumeControlSource<T>
where
    T: Iterator<Item = f32>,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|s| {
            if self.mute.load() {
                0.0
            } else {
                s * self.volume.load()
            }
        })
    }
}

impl<T> Source for VolumeControlSource<T>
where
    T: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
    fn channels(&self) -> u16 {
        self.source.channels()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}
