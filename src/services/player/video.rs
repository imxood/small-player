use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi::av_q2d;
use rsmpeg::ffi::{self, AVRational};
use rsmpeg::swscale::SwsContext;

use crate::services::player::stream::{decode_frame, DecodeContext};
use crate::services::player::VideoFrame;

/// time_base: 从 video stream 中获取到的时间基
pub fn video_decode_thread(mut decode_ctx: DecodeContext, time_base: AVRational) {
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
        // 显示时间, 单位: 秒
        let pts = raw_frame.best_effort_timestamp as f64 * time_base;

        let video = VideoFrame::from(
            rgb_frame.data[0] as *const u8,
            width as usize,
            height as usize,
            rgb_frame.linesize[0] as usize,
            pts,
            duration,
        );

        if let Err(_) = decode_ctx.ctrl.send_video(video) {
            log::info!("video channel disconnected");
            break;
        }
    }

    log::info!("video 解码线程退出");
}
