use std::sync::{atomic::AtomicBool, Arc};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::{
    defines::{AUDIO_FRAME_QUEUE_SIZE, VIDEO_FRAME_QUEUE_SIZE},
    error::Result,
};

use super::{
    audio::{audio_decode_thread, audio_play_thread, AudioFrame},
    demux::{demux_init, demux_thread, DemuxContext},
    video::{video_decode_thread, video_play_thread, VideoFrame},
    Command, PlayState, StreamType,
};

pub fn play(
    filename: String,
    cmd_rx: Receiver<Command>,
    state_tx: Sender<PlayState>,
    abort_request: Arc<AtomicBool>,
) -> Result<()> {
    let (audio_frame_tx, audio_frame_queue) = bounded::<AudioFrame>(AUDIO_FRAME_QUEUE_SIZE);
    let (video_frame_tx, video_frame_queue) = bounded::<VideoFrame>(VIDEO_FRAME_QUEUE_SIZE);

    let (ifmt_ctx, vdec, adec) = demux_init(filename)?;

    let (mut demux_ctx, play_ctrl) = DemuxContext::new(
        ifmt_ctx,
        state_tx,
        audio_frame_tx,
        video_frame_tx,
        abort_request,
    );

    let video_decode_ctx = demux_ctx.build_decode_ctx(vdec, StreamType::Video);
    let audio_decode_ctx = demux_ctx.build_decode_ctx(adec, StreamType::Audio);

    if let Some(decode_ctx) = audio_decode_ctx {
        // 音频解码线程
        let play_ctrl0 = play_ctrl.clone();
        std::thread::spawn(move || {
            audio_decode_thread(play_ctrl0, decode_ctx);
        });

        // 音频播放线程
        let play_ctrl0 = play_ctrl.clone();
        std::thread::spawn(move || {
            if let Err(e) = audio_play_thread(play_ctrl0, audio_frame_queue) {
                log::info!("{}", e.to_string());
            }
        });
    }

    if let Some(decode_ctx) = video_decode_ctx {
        let stream_idx = decode_ctx.stream_idx() as usize;
        let time_base = demux_ctx.stream_time_base(stream_idx)?;
        // 视频解码线程
        let play_ctrl0 = play_ctrl.clone();
        std::thread::spawn(move || {
            video_decode_thread(play_ctrl0, decode_ctx, time_base);
        });

        // 视频播放线程
        let play_ctrl0 = play_ctrl.clone();
        std::thread::spawn(move || {
            if let Err(e) = video_play_thread(play_ctrl0, video_frame_queue) {
                log::info!("{}", e.to_string());
            }
        });
    }

    // 解封装线程
    std::thread::spawn(move || {
        demux_thread(demux_ctx, cmd_rx);
    });

    Ok(())
}
