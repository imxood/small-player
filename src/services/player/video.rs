use std::fmt::Debug;

use crossbeam_channel::Receiver;
use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi::av_q2d;
use rsmpeg::ffi::{self, AVRational};
use rsmpeg::swscale::SwsContext;

use crate::defines::PLAY_MIN_INTERVAL;
use crate::error::Result;
use crate::services::player::stream::{decode_frame, DecodeContext};
use crate::services::player::PlayControl;

use super::PlayFrame;

/// time_base: 从 video stream 中获取到的时间基
pub fn video_decode_thread(
    play_ctrl: PlayControl,
    mut decode_ctx: DecodeContext,
    time_base: AVRational,
) {
    // 获取需要的参数
    let width = decode_ctx.dec_ctx().width;
    let height = decode_ctx.dec_ctx().height;
    let pix_fmt = decode_ctx.dec_ctx().pix_fmt;

    // flags参数选择, 参考: https://blog.csdn.net/leixiaohua1020/article/details/12029505
    let mut video_sws = SwsContext::get_context(
        width,
        height,
        pix_fmt,
        width,
        height,
        ffi::AVPixelFormat_AV_PIX_FMT_RGBA,
        ffi::SWS_FAST_BILINEAR,
    )
    .or_else(|| {
        log::error!("Failed to create a swscale context.");
        None
    })
    .unwrap();

    // 用于把 接收到的帧数据 转换成 特定格式的帧数据
    let mut rgb_frame = AVFrame::new();
    rgb_frame.set_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
    rgb_frame.set_width(width);
    rgb_frame.set_height(height);
    rgb_frame
        .alloc_buffer()
        .map_err(|e| log::error!("frame alloc_buffer failed, error: {:?}", e))
        .unwrap();

    // 帧速率(一秒播放的帧数)的倒数, 即: 一帧的播放的秒数
    let duration = 1.0 / av_q2d(decode_ctx.dec_ctx().framerate);
    // pts dts 等时间参数的基本单位, 表示: 单位1 表示的秒数
    let time_base = av_q2d(time_base);

    loop {
        // 解码视频包, 获取视频帧
        let ret = decode_frame(&play_ctrl, &mut decode_ctx);

        let raw_frame = match ret {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                break;
            }
            Err(e) => {
                log::error!("E: {}", e.to_string());
                break;
            }
        };

        // 视频帧 格式转换
        video_sws
            .scale_frame(&raw_frame, 0, raw_frame.height, &mut rgb_frame)
            .map_err(|e| log::error!("video sws scale_frame failed, error: {:?}", e))
            .unwrap();

        // best_effort_timestamp 它是以时间基为单位, 表示 best_effort_timestamp 个时间基.
        // 显示时间, 单位: 秒
        let pts = raw_frame.best_effort_timestamp as f64 * time_base;

        // 从新格式的视频帧中 构建rgb数据
        let video = VideoFrame::new(
            rgb_frame.data[0] as *const u8,
            width as usize,
            height as usize,
            rgb_frame.linesize[0] as usize,
            pts,
            duration,
        );

        // 发送 rgb数据 给 video play thread
        if let Err(_) = play_ctrl.send_video(video) {
            log::info!("video channel disconnected");
            break;
        }
    }
    log::info!("视频解码线程退出");
}

pub fn video_play_thread(
    play_ctrl: super::PlayControl,
    video_frame_queue: Receiver<VideoFrame>,
) -> Result<()> {
    let mut empty_count = 0;

    loop {
        if play_ctrl.abort_request() {
            break;
        }

        if play_ctrl.pause() {
            play_ctrl.wait_notify_in_pause();
        }

        if let Ok(frame) = video_frame_queue.try_recv() {
            play_ctrl.play_video(frame)?;
            empty_count = 0;
            continue;
        }

        empty_count += 1;
        if empty_count == 10 {
            play_ctrl.set_video_finished(true);
            break;
        }
        spin_sleep::sleep(PLAY_MIN_INTERVAL);
    }
    log::info!("视频播放线程退出");
    Ok(())
}

#[derive(Default, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub pts: f64,
    pub duration: f64,
}

impl VideoFrame {
    pub fn new(
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

impl PlayFrame for VideoFrame {
    fn pts(&self) -> f64 {
        self.pts
    }

    fn duration(&self) -> f64 {
        self.duration
    }

    fn mem_size(&self) -> usize {
        self.data.len()
    }
}

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame")
            // .field("width", &self.width)
            // .field("height", &self.height)
            .field("pts", &self.pts)
            .field("duration", &self.duration)
            .finish()
    }
}
