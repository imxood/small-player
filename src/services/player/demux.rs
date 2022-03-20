use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TryRecvError};

use crate::services::player::decode::DemuxContext;
use crate::services::player::{Command, PlayState, StreamType};

pub fn demux_thread(
    mut demux_ctx: DemuxContext,
    cmd_rx: Receiver<Command>,
    state_tx: Sender<PlayState>,
) {
    let (video_stream_idx, audio_stream_idx) = demux_ctx.stream_idx();
    loop {
        match cmd_rx.try_recv() {
            Ok(Command::Stop) => {
                demux_ctx.ctrl.set_abort_request(true);
                state_tx.send(PlayState::Stopped).ok();
                log::info!("run abort_request cmd");
                break;
            }
            Ok(Command::Pause) => {
                log::info!("run pause cmd");
                demux_ctx.ctrl.set_pause(true);
                state_tx.send(PlayState::Pausing).ok();
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
            && demux_ctx.queue_is_empty(StreamType::Video)
            && demux_ctx.queue_is_empty(StreamType::Audio)
        {
            demux_ctx.ctrl.set_abort_request(true);
            log::info!("decode finished");
            break;
        }

        match demux_ctx.read_packet() {
            Ok(Some(pkt)) => {
                // 如果是 视频数据包
                if pkt.stream_index == video_stream_idx {
                    demux_ctx.queue_push(pkt, StreamType::Video);
                }
                // 如果是 音频数据包
                // else if pkt.stream_index == audio_stream_idx {
                //     demux_ctx.queue_push(pkt, StreamType::Audio);
                // }
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