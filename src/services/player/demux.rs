use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::{ffi::CString, sync::Arc};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use parking_lot::{Mutex, RwLock};
use rsmpeg::ffi::AVRational;
use rsmpeg::{
    avcodec::{AVCodecContext, AVPacket},
    avformat::AVFormatContextInput,
    ffi,
};

use crate::error::{PlayerError, Result};
use crate::services::player::{Command, StreamType};

use super::{
    audio::{AudioDevice, AudioFrame},
    stream::DecodeContext,
    video::VideoFrame,
    PacketQueue, PlayControl, PlayState,
};

pub fn demux_thread(mut demux_ctx: DemuxContext, cmd_rx: Receiver<Command>) {
    let (video_stream_idx, audio_stream_idx) = demux_ctx.stream_idx();
    loop {
        match cmd_rx.try_recv() {
            Ok(Command::Terminate) => {
                log::info!("run abort_request cmd");
                demux_ctx.ctrl.set_abort_request(true);
                break;
            }
            Ok(Command::Pause(pause)) => {
                log::info!("run pause cmd: {pause}");
                demux_ctx.ctrl.set_pause(pause);
            }
            Ok(Command::Mute(mute)) => {
                log::info!("recv mute command: {mute}");
                demux_ctx.ctrl.set_mute(mute);
            }
            Ok(Command::Volume(volume)) => {
                log::info!("recv volume command: {volume}");
                demux_ctx.ctrl.set_volume(volume);
            }
            Err(TryRecvError::Disconnected) => {
                demux_ctx.ctrl.set_abort_request(true);
                log::info!("demux_thread disconnected");
                break;
            }
            Err(TryRecvError::Empty) => {}
        }

        // 暂停 / 声音队列已满 / 视频队列已满
        if demux_ctx.ctrl.pause()
            || demux_ctx.queue_is_full(StreamType::Audio)
            || demux_ctx.queue_is_full(StreamType::Video)
        {
            spin_sleep::sleep(Duration::from_millis(20));
            continue;
        }

        // 解封装完成 且队列为空, 则退出
        if demux_ctx.ctrl.demux_finished()
            && demux_ctx.ctrl.audio_finished()
            && demux_ctx.ctrl.video_finished()
            && demux_ctx.queue_is_empty(StreamType::Video)
            && demux_ctx.queue_is_empty(StreamType::Audio)
        {
            demux_ctx.ctrl.set_abort_request(true);
            break;
        }

        match demux_ctx.read_packet() {
            Ok(Some(pkt)) => {
                // 视频数据包
                if pkt.stream_index == video_stream_idx {
                    demux_ctx.queue_push(pkt, StreamType::Video);
                }
                // 音频数据包
                else if pkt.stream_index == audio_stream_idx {
                    demux_ctx.queue_push(pkt, StreamType::Audio);
                }
            }
            Ok(None) => {
                demux_ctx.ctrl.set_demux_finished(true);
                spin_sleep::sleep(Duration::from_millis(20));
            }
            Err(e) => log::error!("Read frame error: {:?}", e),
        };
    }
    log::info!("解封装线程退出");
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
        vdec_ctx.apply_codecpar(&av_stream.codecpar())?;
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
            adec_ctx.apply_codecpar(&av_stream.codecpar())?;
        }
        adec_ctx.open(None)?;
        Some((stream_idx, adec_ctx))
    } else {
        None
    };

    Ok((ifmt_ctx, vdec, adec))
}

pub struct DemuxContext {
    ctrl: PlayControl,
    ifmt_ctx: AVFormatContextInput,
    video_queue: Arc<Mutex<PacketQueue>>,
    audio_queue: Arc<Mutex<PacketQueue>>,
}

impl DemuxContext {
    /// 无效的流索引
    pub const UNKNOWN_STREAM_IDX: i32 = -1;
    pub const MAX_MEM_SIZE: i32 = 16 * 1024 * 1024;

    pub fn new(
        ifmt_ctx: AVFormatContextInput,
        state_tx: Sender<PlayState>,
        audio_frame_tx: Sender<AudioFrame>,
        video_frame_tx: Sender<VideoFrame>,
        abort_request: Arc<AtomicBool>,
    ) -> (Self, PlayControl) {
        let video_queue = Arc::new(Mutex::new(PacketQueue::new(
            Self::UNKNOWN_STREAM_IDX,
            Self::MAX_MEM_SIZE,
        )));
        let audio_queue = Arc::new(Mutex::new(PacketQueue::new(
            Self::UNKNOWN_STREAM_IDX,
            Self::MAX_MEM_SIZE,
        )));

        // 获取音频设备
        let audio_dev = AudioDevice::new()
            .map_err(|e| {
                state_tx.send(PlayState::Error(e)).ok();
            })
            .unwrap();
        let audio_dev = Arc::new(RwLock::new(audio_dev));

        // 控制播放器的行为
        let ctrl = PlayControl::new(
            audio_dev,
            state_tx,
            audio_frame_tx,
            video_frame_tx,
            abort_request,
        );
        let ctrl0 = ctrl.clone();

        (
            Self {
                ifmt_ctx,
                ctrl,
                video_queue,
                audio_queue,
            },
            ctrl0,
        )
    }

    pub fn stream_time_base(&self, stream_idx: usize) -> Result<AVRational> {
        let time_base = self
            .ifmt_ctx
            .streams()
            .get(stream_idx)
            .ok_or_else(|| PlayerError::Error(format!("根据 stream_idx 无法获取到 stream")))?
            .time_base;
        Ok(time_base)
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

            let decode_ctx = DecodeContext::new(dec_ctx, packet_queue.clone());

            *self.queue_mut(stream_type) = packet_queue;

            Some(decode_ctx)
        } else {
            None
        }
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
