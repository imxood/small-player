use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use std::{collections::LinkedList, sync::atomic::Ordering};

use crossbeam_channel::{SendError, Sender, TrySendError};
use parking_lot::RwLock;
use rsmpeg::avcodec::AVPacket;

use crate::error::PlayerError;

use self::audio::{AudioDevice, AudioFrame};

pub mod audio;
pub mod decode;
pub mod demux;
pub mod play;
pub mod stream;
pub mod video;

pub enum Command {
    Stop,
    Pause(bool),
    Mute(bool),
    Volume(f32),
}

// Send + Sync + Clone + Eq + Debug + Hash
#[derive(Debug, Clone)]
pub enum PlayState {
    Start,
    Loading,
    Playing,
    Pausing(bool),
    Stopped,
    Video(VideoFrame),
    Error(PlayerError),
}

impl Default for PlayState {
    fn default() -> Self {
        Self::Start
    }
}

impl PartialEq for PlayState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Video(_), Self::Video(_)) => true,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Default, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub pts: f64,
    pub duration: f64,
}

impl ToString for VideoFrame {
    fn to_string(&self) -> String {
        format!(
            "VideoFrame: len {}, width {}, height {}",
            self.data.len(),
            self.width,
            self.height,
        )
    }
}

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pts", &self.pts)
            .field("duration", &self.duration)
            .finish()
    }
}

impl VideoFrame {
    pub fn from(
        raw_data: *const u8,
        width: usize,
        height: usize,
        line_size: usize,
        pts: f64,
        duration: f64,
    ) -> Self {
        let raw_data = unsafe { std::slice::from_raw_parts(raw_data, height * line_size) };
        let mut data: Vec<u8> = vec![0; width * height * 4];
        let data_slice = data.as_mut_slice();
        for i in 0..height as usize {
            let start = i * width * 4;
            let end = start + width * 4;
            let slice = &mut data_slice[start..end];

            let start = i * line_size;
            let end = start + width * 4;
            slice.copy_from_slice(&raw_data[start..end]);
        }
        Self {
            data,
            width,
            height,
            pts,
            duration,
        }
    }
}

pub enum StreamType {
    Video,
    Audio,
}

#[derive(Clone)]
pub struct PlayControl {
    state_tx: Sender<PlayState>,
    abort_request: Arc<AtomicBool>,
    demux_finished: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
    audio_dev: Arc<RwLock<AudioDevice>>,
    volume: Arc<RwLock<f32>>,
    audio_frame_tx: Sender<AudioFrame>,
    video_frame_tx: Sender<VideoFrame>,
}

impl PlayControl {
    pub fn new(
        audio_dev: Arc<RwLock<AudioDevice>>,
        state_tx: Sender<PlayState>,
        audio_frame_tx: Sender<AudioFrame>,
        video_frame_tx: Sender<VideoFrame>,
    ) -> Self {
        let abort_request = Arc::new(AtomicBool::new(false));
        let demux_finished = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let playing = Arc::new(AtomicBool::new(true));
        Self {
            state_tx,
            abort_request,
            demux_finished,
            pause,
            playing,
            audio_dev,
            audio_frame_tx,
            video_frame_tx,
            volume: Arc::new(RwLock::new(1.0)),
        }
    }

    pub fn set_mute(&self, mute: bool) {
        self.audio_dev.write().set_mute(mute);
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.write() = volume;
    }

    pub fn volume(&self) -> f32 {
        *self.volume.read()
    }

    pub fn set_abort_request(&self, abort_request: bool) {
        self.abort_request.store(abort_request, Ordering::Relaxed);
        self.audio_dev.write().stop();
        self.state_tx.send(PlayState::Stopped).ok();
    }

    pub fn abort_request(&self) -> bool {
        self.abort_request.load(Ordering::Relaxed)
    }

    pub fn set_pause(&mut self, pause: bool) {
        self.pause.store(pause, Ordering::Relaxed);
        self.audio_dev.write().set_pause(pause);
        self.state_tx.send(PlayState::Pausing(pause)).ok();
    }

    pub fn pause(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_demux_finished(&self, demux_finished: bool) {
        self.demux_finished.store(demux_finished, Ordering::Relaxed);
    }

    pub fn demux_finished(&self) -> bool {
        self.demux_finished.load(Ordering::Relaxed)
    }

    pub fn audio_default_config(&self) -> cpal::SupportedStreamConfig {
        self.audio_dev.read().default_config()
    }

    pub fn send_audio(&self, audio: AudioFrame) -> Result<(), SendError<AudioFrame>> {
        self.audio_frame_tx.send(audio)
    }

    pub fn send_video(&self, video: VideoFrame) -> Result<(), SendError<VideoFrame>> {
        self.video_frame_tx.send(video)
    }

    pub fn send_state(&self, state: PlayState) -> Result<(), TrySendError<PlayState>> {
        self.state_tx.try_send(state)
    }

    pub fn play_audio(&self, audio_source: AudioFrame) {
        self.audio_dev.write().play_source(audio_source);
    }
}

pub struct PacketQueue {
    queue: LinkedList<AVPacket>,
    mem_size: i32,
    max_mem_size: i32,
    stream_idx: i32,
}

impl PacketQueue {
    pub fn new(stream_idx: i32, max_mem_size: i32) -> Self {
        Self {
            queue: LinkedList::new(),
            mem_size: 0,
            max_mem_size,
            stream_idx,
        }
    }

    pub fn mem_size(&self) -> i32 {
        self.mem_size
    }

    pub fn stream_idx(&self) -> i32 {
        self.stream_idx
    }

    pub fn set_max_mem_size(&mut self, max_mem_size: i32) {
        self.max_mem_size = max_mem_size;
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.mem_size >= self.max_mem_size
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn push(&mut self, pkt: AVPacket) {
        self.mem_size += pkt.size;
        self.queue.push_back(pkt);
    }

    pub fn pop(&mut self) -> Option<AVPacket> {
        let pkt = self.queue.pop_front();
        if let Some(pkt) = pkt.as_ref() {
            self.mem_size -= pkt.size;
        }
        pkt
    }
}
