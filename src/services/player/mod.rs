use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{collections::LinkedList, sync::atomic::Ordering};

use parking_lot::{Mutex, MutexGuard};
use rsmpeg::avcodec::AVPacket;

pub mod decode;
pub mod demux;
pub mod stream;

pub enum Command {
    Stop,
    Pause,
}

// Send + Sync + Clone + Eq + Debug + Hash
#[derive(Debug, Clone)]
pub enum PlayState {
    Start,
    Loading,
    Playing,
    Pausing,
    Stopped,
    Video(VideoFrame),
    Audio,
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
            "VideoFrame: len {}, width {}, height {}, pts {}, duration {}",
            self.data.len(),
            self.width,
            self.height,
            self.pts,
            self.duration,
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

pub struct AudioFrame {}

pub enum StreamType {
    Video,
    Audio,
}

#[derive(Clone)]
pub struct PlayControl {
    abort_request: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    demux_finished: Arc<AtomicBool>,
    video_finished: Arc<AtomicBool>,
    audio_finished: Arc<AtomicBool>,
}

impl PlayControl {
    pub fn new() -> Self {
        let abort_request = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let demux_finished = Arc::new(AtomicBool::new(false));
        // 视频解码和音频解码 默认是已完成
        // 在实际处理时, 在解封装期间 解析到了 packet 时, 会更新为 false
        let video_finished = Arc::new(AtomicBool::new(true));
        let audio_finished = Arc::new(AtomicBool::new(true));
        // 如果 解封装 视频解码 音频解码 都成功, 那么
        Self {
            pause,
            abort_request,
            demux_finished,
            video_finished,
            audio_finished,
        }
    }

    pub fn set_abort_request(&self, abort_request: bool) {
        self.abort_request.store(abort_request, Ordering::Relaxed);
    }

    pub fn abort_request(&self) -> bool {
        self.abort_request.load(Ordering::Relaxed)
    }

    pub fn set_pause(&self, pause: bool) {
        self.pause.store(pause, Ordering::Relaxed);
    }

    pub fn pause(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }

    pub fn set_demux_finished(&self, demux_finished: bool) {
        self.demux_finished.store(demux_finished, Ordering::Relaxed);
    }

    pub fn demux_finished(&self) -> bool {
        self.demux_finished.load(Ordering::Relaxed)
    }

    // pub fn set_video_finished(&self, video_finished: bool) {
    //     self.video_finished.store(video_finished, Ordering::Relaxed);
    // }

    // pub fn video_finished(&self) -> bool {
    //     self.video_finished.load(Ordering::Relaxed)
    // }

    // pub fn set_audio_finished(&self, audio_finished: bool) {
    //     self.audio_finished.store(audio_finished, Ordering::Relaxed);
    // }

    // pub fn audio_finished(&self) -> bool {
    //     self.audio_finished.load(Ordering::Relaxed)
    // }

    // pub fn is_finished(&self) -> bool {
    //     self.demux_finished() && self.video_finished() && self.audio_finished()
    // }
}

#[derive(Clone)]
struct LockedPacketQueue(Arc<Mutex<PacketQueue>>);

impl LockedPacketQueue {
    // pub fn new(stream_idx: i32) -> Self {
    //     Self(Arc::new(Mutex::new(PacketQueue::new(stream_idx))))
    // }

    pub fn stream_idx(&self) -> i32 {
        self.0.lock().stream_idx
    }

    pub fn set_max_mem_size(&self, max_size: i32) {
        self.0.lock().set_max_mem_size(max_size);
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.0.lock().is_full()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.lock().is_empty()
    }

    /// 向结尾追加一个包
    pub fn push(&mut self, pkt: AVPacket) {
        self.0.lock().push(pkt);
    }

    // 从开头取出一个包
    pub fn pop(&self) -> Option<AVPacket> {
        self.0.lock().pop()
    }

    pub fn lock(&self) -> MutexGuard<PacketQueue> {
        self.0.lock()
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
        log::info!("pkt.size: {:5} mem_size: {:8}", pkt.size, self.mem_size);
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
