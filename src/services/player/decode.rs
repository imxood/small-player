use std::{ffi::CString, sync::Arc};

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;
use rsmpeg::{
    avcodec::{AVCodecContext, AVPacket},
    avformat::AVFormatContextInput,
    ffi,
};

use crate::error::{PlayerError, Result};

use super::{
    audio::audio_decode_thread, demux::demux_thread, stream::DecodeContext,
    video::video_decode_thread, Command, PacketQueue, PlayControl, PlayState, StreamType,
};

pub fn decode(
    filename: String,
    cmd_rx: Receiver<Command>,
    state_tx: Sender<PlayState>,
) -> Result<()> {
    let (ifmt_ctx, vdec, adec) = demux_init(filename)?;

    let mut demux_ctx = DemuxContext::new(ifmt_ctx);

    let video_decode_ctx = demux_ctx.build_decode_ctx(vdec, StreamType::Video);
    let audio_decode_ctx = demux_ctx.build_decode_ctx(adec, StreamType::Audio);

    // 视频解码线程
    if let Some(decode_ctx) = video_decode_ctx {
        let state_tx = state_tx.clone();
        let stream_idx = decode_ctx.stream_idx() as usize;
        let av_stream = demux_ctx
            .ifmt_ctx_mut()
            .streams()
            .get(stream_idx)
            .ok_or_else(|| {
                PlayerError::Error(format!("根据 video stream_idx 无法获取到 video stream"))
            })?;
        let time_base = av_stream.time_base;
        std::thread::spawn(move || {
            video_decode_thread(decode_ctx, state_tx, time_base);
        });
    }

    // 音频解码线程
    if let Some(decode_ctx) = audio_decode_ctx {
        let state_tx = state_tx.clone();
        std::thread::spawn(move || {
            audio_decode_thread(decode_ctx, state_tx);
        });
    }

    // 解封装线程
    std::thread::spawn(move || {
        let state_tx = state_tx.clone();
        demux_thread(demux_ctx, cmd_rx, state_tx);
    });

    Ok(())
}

pub fn demux_init(
    filename: String,
) -> Result<(
    AVFormatContextInput,
    Option<(usize, AVCodecContext)>,
    Option<(usize, AVCodecContext)>,
)> {
    let filename = CString::new(filename)?;
    // 获取输入流的上下文
    let ifmt_ctx = AVFormatContextInput::open(&filename).map_err(|e| {
        PlayerError::Error(format!(
            " AVFormatContextInput::open filename({:?}), E: {}",
            &filename,
            e.to_string()
        ))
    })?;

    // 获取视频解码器
    let video_decoder = ifmt_ctx
        .find_best_stream(ffi::AVMediaType_AVMEDIA_TYPE_VIDEO)
        .map_err(|e| {
            PlayerError::Error(format!(
                "find_best_stream video failed, E: {}",
                e.to_string()
            ))
        })?;
    let vdec = if let Some((stream_idx, decoder)) = video_decoder {
        let mut vdec_ctx = AVCodecContext::new(&decoder);
        let av_stream = ifmt_ctx.streams().get(stream_idx).ok_or_else(|| {
            PlayerError::Error(format!("根据 video stream_idx 无法获取到 video stream"))
        })?;
        vdec_ctx.apply_codecpar(av_stream.codecpar())?;
        vdec_ctx.set_framerate(av_stream.guess_framerate().unwrap());

        vdec_ctx.open(None)?;

        Some((stream_idx, vdec_ctx))
    } else {
        None
    };

    // 获取音频解码器
    let audio_decoder = ifmt_ctx
        .find_best_stream(ffi::AVMediaType_AVMEDIA_TYPE_AUDIO)
        .map_err(|e| {
            PlayerError::Error(format!(
                "find_best_stream audio failed, E: {}",
                e.to_string()
            ))
        })?;
    let adec = if let Some((stream_idx, decoder)) = audio_decoder {
        let mut adec_ctx = AVCodecContext::new(&decoder);
        {
            let av_stream = ifmt_ctx.streams().get(stream_idx).ok_or_else(|| {
                PlayerError::Error(format!("根据 audio stream_idx 无法获取到 audio stream"))
            })?;
            adec_ctx.apply_codecpar(av_stream.codecpar())?;
        }
        adec_ctx.open(None)?;
        Some((stream_idx, adec_ctx))
    } else {
        None
    };

    Ok((ifmt_ctx, vdec, adec))
}

pub struct DemuxContext {
    pub ctrl: PlayControl,
    ifmt_ctx: AVFormatContextInput,
    video_queue: Arc<Mutex<PacketQueue>>,
    audio_queue: Arc<Mutex<PacketQueue>>,
}

impl DemuxContext {
    /// 无效的流索引
    pub const UNKNOWN_STREAM_IDX: i32 = -1;
    pub const MAX_MEM_SIZE: i32 = 16 * 1024 * 1024;

    pub fn new(ifmt_ctx: AVFormatContextInput) -> Self {
        let video_queue = Arc::new(Mutex::new(PacketQueue::new(
            Self::UNKNOWN_STREAM_IDX,
            Self::MAX_MEM_SIZE,
        )));
        let audio_queue = Arc::new(Mutex::new(PacketQueue::new(
            Self::UNKNOWN_STREAM_IDX,
            Self::MAX_MEM_SIZE,
        )));
        Self {
            ifmt_ctx,
            ctrl: PlayControl::new(),
            video_queue,
            audio_queue,
        }
    }

    pub fn read_packet(
        &mut self,
    ) -> std::result::Result<Option<AVPacket>, rsmpeg::error::RsmpegError> {
        self.ifmt_ctx.read_packet()
    }

    pub fn build_decode_ctx(
        &mut self,
        decode: Option<(usize, AVCodecContext)>,
        stream_type: StreamType,
    ) -> Option<DecodeContext> {
        if let Some((stream_idx, dec_ctx)) = decode {
            let stream_idx = stream_idx as i32;

            let packet_queue =
                Arc::new(Mutex::new(PacketQueue::new(stream_idx, Self::MAX_MEM_SIZE)));

            let decode_ctx = DecodeContext::new(dec_ctx, self.ctrl.clone(), packet_queue.clone());

            *self.queue_mut(stream_type) = packet_queue;

            Some(decode_ctx)
        } else {
            None
        }
    }

    pub fn ifmt_ctx_mut(&mut self) -> &mut AVFormatContextInput {
        &mut self.ifmt_ctx
    }

    /// return (video_stream_id, audio_stream_id)
    pub fn stream_idx(&self) -> (i32, i32) {
        (
            self.video_queue.lock().stream_idx(),
            self.audio_queue.lock().stream_idx(),
        )
    }

    pub fn queue_push(&self, pkt: AVPacket, stream_type: StreamType) {
        self.queue(stream_type).lock().push(pkt);
    }

    pub fn queue_is_full(&self, stream_type: StreamType) -> bool {
        self.queue(stream_type).lock().is_full()
    }

    pub fn queue_is_empty(&self, stream_type: StreamType) -> bool {
        self.queue(stream_type).lock().is_empty()
    }

    fn queue(&self, stream_type: StreamType) -> &Arc<Mutex<PacketQueue>> {
        match stream_type {
            StreamType::Video => &self.video_queue,
            StreamType::Audio => &self.audio_queue,
        }
    }

    fn queue_mut(&mut self, stream_type: StreamType) -> &mut Arc<Mutex<PacketQueue>> {
        match stream_type {
            StreamType::Video => &mut self.video_queue,
            StreamType::Audio => &mut self.audio_queue,
        }
    }
}

unsafe impl Send for DemuxContext {}
