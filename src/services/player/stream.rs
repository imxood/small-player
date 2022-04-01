use parking_lot::Mutex;
use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::avcodec::AVPacket;
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi::{self};
use std::sync::Arc;
use std::time::Duration;

use super::{PacketQueue, PlayControl};
use crate::error::{PlayerError, Result};

pub struct DecodeContext {
    dec_ctx: AVCodecContext,
    queue: Arc<Mutex<PacketQueue>>,
}

impl DecodeContext {
    pub fn new(dec_ctx: AVCodecContext, queue: Arc<Mutex<PacketQueue>>) -> Self {
        Self { dec_ctx, queue }
    }

    pub fn stream_idx(&self) -> i32 {
        self.queue.lock().stream_idx()
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

pub fn decode_frame(
    play_ctrl: &PlayControl,
    decode_ctx: &mut DecodeContext,
) -> Result<Option<AVFrame>> {
    let mut retry_send_packet;
    loop {
        retry_send_packet = false;
        // 先尝试 接收一帧, 后面再判断是否需要退出, 原因是为了避免最后一帧数据丢失
        match decode_ctx.dec_ctx_mut().receive_frame() {
            Ok(frame) => {
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

        loop {
            if play_ctrl.abort_request() {
                return Ok(None);
            }
            // 已暂停 / Packet中没有数据
            if !play_ctrl.pause() || !decode_ctx.queue_is_empty() {
                break;
            }
            spin_sleep::sleep(Duration::from_millis(50));
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
