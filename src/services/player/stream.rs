use crossbeam_channel::Sender;
use parking_lot::Mutex;
use rsmpeg::avcodec::AVPacket;
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi::{self, av_q2d};
use rsmpeg::{avcodec::AVCodecContext, swscale::SwsContext};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{PacketQueue, PlayControl};
use crate::error::{PlayerError, Result};
use crate::services::player::{PlayState, VideoFrame};

pub struct DecodeContext {
    pub ctrl: PlayControl,
    dec_ctx: AVCodecContext,
    queue: Arc<Mutex<PacketQueue>>,
}

impl DecodeContext {
    pub fn new(dec_ctx: AVCodecContext, ctrl: PlayControl, queue: Arc<Mutex<PacketQueue>>) -> Self {
        Self {
            dec_ctx,
            ctrl,
            queue,
        }
    }

    pub fn queue_pop(&self) -> Option<AVPacket> {
        self.queue.lock().pop()
    }

    pub fn queue_is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    pub fn dec_ctx_mut(&mut self) -> &mut AVCodecContext {
        &mut self.dec_ctx
    }

    pub fn dec_ctx(&mut self) -> &mut AVCodecContext {
        &mut self.dec_ctx
    }
}

pub fn video_decode_frame(decode_ctx: &mut DecodeContext) -> Result<Option<AVFrame>> {
    let mut retry_send_packet;
    loop {
        retry_send_packet = false;
        // 先尝试 接收一帧, 后面再判断是否需要退出, 原因是为了避免最后一帧数据丢失
        match decode_ctx.dec_ctx_mut().receive_frame() {
            Ok(mut frame) => {
                unsafe { (*frame.as_mut_ptr()).pts = frame.best_effort_timestamp };
                return Ok(Some(frame));
            }
            Err(RsmpegError::DecoderDrainError) => {
                retry_send_packet = true;
            }
            Err(RsmpegError::DecoderFlushedError) => unsafe {
                ffi::avcodec_flush_buffers(decode_ctx.dec_ctx().as_mut_ptr());
                return Ok(None);
            },
            Err(e) => return Err(PlayerError::Error(e.to_string())),
        }

        if decode_ctx.ctrl.abort_request() {
            return Ok(None);
        }
        // 已暂停 / Packet中没有数据
        if decode_ctx.ctrl.pause() || decode_ctx.queue_is_empty() {
            spin_sleep::sleep(Duration::from_millis(20));
            continue;
        }

        if retry_send_packet {
            let pkt = decode_ctx.queue_pop();
            if let Some(pkt) = pkt {
                if pkt.data as *const u8 == std::ptr::null() {
                    unsafe { ffi::avcodec_flush_buffers(decode_ctx.dec_ctx().as_mut_ptr()) };
                    continue;
                }
                // 将packet发送给解码器
                //  发送packet的顺序是按dts递增的顺序，如IPBBPBB
                //  pkt.pos变量可以标识当前packet在视频文件中的地址偏移
                match decode_ctx.dec_ctx().send_packet(Some(&pkt)) {
                    Ok(_) => {}
                    Err(RsmpegError::DecoderFlushedError | RsmpegError::DecoderFullError) => {}
                    Err(e) => {
                        log::error!("video dec send_packet failed, E: {}", e.to_string());
                    }
                }
            } else {
                spin_sleep::sleep(Duration::from_millis(20));
            }
        }
    }
}

pub fn video_decode_thread(mut decode_ctx: DecodeContext, state_tx: Sender<PlayState>) {
    // 获取需要的参数
    let width = decode_ctx.dec_ctx().width;
    let height = decode_ctx.dec_ctx().height;
    let pix_fmt = decode_ctx.dec_ctx().pix_fmt;
    let sample_aspect_ratio = decode_ctx.dec_ctx().sample_aspect_ratio;
    let time_base = decode_ctx.dec_ctx().time_base;
    let framerate = decode_ctx.dec_ctx().framerate;

    // 一帧的播放时长
    let duration = av_q2d(framerate);
    // 时间基, 单位: 秒
    let timebase = av_q2d(time_base);

    log::info!("duration: {duration}");

    let mut video_sws = SwsContext::get_context(
        width,
        height,
        pix_fmt,
        width,
        height,
        ffi::AVPixelFormat_AV_PIX_FMT_RGBA,
        ffi::SWS_BILINEAR,
    )
    .or_else(|| {
        log::error!("Failed to create a swscale context.");
        None
    })
    .unwrap();

    log::info!("sample_aspect_ratio: {:?}", sample_aspect_ratio);

    let mut rgb_frame = AVFrame::new();
    rgb_frame.set_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
    rgb_frame.set_width(width);
    rgb_frame.set_height(height);
    rgb_frame
        .alloc_buffer()
        .map_err(|e| log::error!("frame alloc_buffer failed, error: {:?}", e))
        .unwrap();

    let now = Instant::now();

    loop {
        let frame = video_decode_frame(&mut decode_ctx);
        let raw_frame = match frame {
            Ok(None) => {
                log::info!("video decode exited");
                break;
            }
            Ok(Some(frame)) => frame,
            Err(e) => {
                log::error!("E: {}", e.to_string());
                break;
            }
        };

        // 格式转换
        video_sws
            .scale_frame(&raw_frame, 0, raw_frame.height, &mut rgb_frame)
            .map_err(|e| log::error!("video sws scale_frame failed, error: {:?}", e))
            .unwrap();

        let pts = raw_frame.pts as f64 * timebase;

        let video_data = VideoFrame::from(
            rgb_frame.data[0] as *const u8,
            width as usize,
            height as usize,
            rgb_frame.linesize[0] as usize,
            pts,
            duration,
        );

        state_tx.send(PlayState::Video(video_data)).ok();
        log::info!("sended");
    }

    log::info!("video解码线程退出");
    // video_thread_ctx.terminal().unwrap_or_log();
}

pub fn audio_decode_thread(_decode_ctx: DecodeContext, _state_tx: Sender<PlayState>) {}
