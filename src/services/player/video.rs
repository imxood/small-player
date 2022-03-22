use std::time::{Duration, Instant};

use crossbeam_channel::{Sender, TrySendError};
use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi::av_q2d;
use rsmpeg::ffi::{self, AVRational};
use rsmpeg::swscale::SwsContext;

use crate::services::player::stream::{decode_frame, DecodeContext};
use crate::services::player::{PlayState, VideoFrame};

/// time_base: 从 video stream 中获取到的时间基
pub fn video_decode_thread(
    mut decode_ctx: DecodeContext,
    state_tx: Sender<PlayState>,
    time_base: AVRational,
) {
    // 获取需要的参数
    let width = decode_ctx.dec_ctx().width;
    let height = decode_ctx.dec_ctx().height;
    let pix_fmt = decode_ctx.dec_ctx().pix_fmt;

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

    let mut rgb_frame = AVFrame::new();
    rgb_frame.set_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
    rgb_frame.set_width(width);
    rgb_frame.set_height(height);
    rgb_frame
        .alloc_buffer()
        .map_err(|e| log::error!("frame alloc_buffer failed, error: {:?}", e))
        .unwrap();

    // 一帧的播放时长
    let duration = 1.0 / av_q2d(decode_ctx.dec_ctx().framerate);
    // 时间基, 单位: 秒
    let time_base = av_q2d(time_base);

    let start = Instant::now();
    let mut first_time = Option::<Duration>::None;
    let mut pts = 0.0;
    let mut new_pts = 0.0;

    loop {
        let frame = decode_frame(&mut decode_ctx);
        log::debug!(
            "video queue mem_size: {} MByte",
            decode_ctx.queue_mem_size() as f32 / 1000000.0
        );

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

        // best_effort_timestamp 它是以时间基为单位, 表示 best_effort_timestamp 个时间基.
        // 显示时间

        // 取出一帧播放, 预取下一帧后, 再根据 pts(显示时间) 和 duration(间隔), 休眠一定的时间
        // 预取操作可以让当前帧播放结束后, 下一帧立即播放, 时间误差会极小
        loop {
            // 新的frame, pts变成了 new_pts
            new_pts = raw_frame.best_effort_timestamp as f64 * time_base;
            if first_time.is_none() {
                pts = new_pts;
                let video_data = VideoFrame::from(
                    rgb_frame.data[0] as *const u8,
                    width as usize,
                    height as usize,
                    rgb_frame.linesize[0] as usize,
                    pts,
                    duration,
                );
                log::info!("video pts:{:?}s, time_base:{}", pts, time_base);
                match state_tx.try_send(PlayState::Video(video_data)) {
                    Ok(_) | Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => {
                        log::info!("video decode thread disconnected");
                        log::info!("video解码线程退出");
                        return;
                    }
                }
                first_time = Some(start.elapsed());
                break;
            } else {
                let duration = Duration::from_secs_f64(duration);
                let pts = Duration::from_secs_f64(pts);
                let elapsed = start.elapsed() - first_time.take().unwrap();
                // 暂停会导致 elapsed 太大
                if elapsed > duration {
                    continue;
                }
                // 剩余的延时时间
                let duration = duration - elapsed;
                // 当前 帧的显示时间 要在合适的范围
                let cur = start.elapsed();
                if pts >= cur && pts <= cur + duration {
                    spin_sleep::sleep(duration);
                }
            }
        }
    }

    log::info!("video解码线程退出");
}
