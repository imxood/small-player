use std::{
    collections::LinkedList,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, TrySendError};

use crate::defines::{PLAY_MAX_INTERVAL, PLAY_MIN_INTERVAL};

use super::{audio::AudioFrame, PlayControl, PlayState, VideoFrame};

#[derive(Debug)]
pub enum PlayFrame {
    Audio(AudioFrame),
    Video(VideoFrame),
}

impl PlayFrame {
    pub fn mem_size(&self) -> usize {
        match self {
            PlayFrame::Audio(audio) => audio.mem_size(),
            PlayFrame::Video(video) => video.mem_size(),
        }
    }

    fn time_info(&self) -> (f64, f64) {
        match &self {
            PlayFrame::Audio(a) => (a.pts, a.duration),
            PlayFrame::Video(v) => (v.pts, v.duration),
        }
    }
}

pub fn play(
    ctrl: PlayControl,
    audio_frame_queue: Receiver<AudioFrame>,
    video_frame_queue: Receiver<VideoFrame>,
) {
    let start = Instant::now();

    let mut count = 0;
    // 一个组合帧: 一个视频帧 + 若干个音频帧, 一次播放的数据, 播放后将休眠当前线程
    let mut play_queue = PlayQueue::new();
    // 播放结束时间, 用于计算上一帧播放结束到下一帧开始播放时, 经历的时间
    let mut play_end_time = start.elapsed();

    loop {
        while let Ok(frame) = audio_frame_queue.try_recv() {
            log::info!("{:?}", &frame);
            play_queue.insert(PlayFrame::Audio(frame));
        }
        while let Ok(frame) = video_frame_queue.try_recv() {
            // log::info!("{:?}", &frame);
            play_queue.insert(PlayFrame::Video(frame));
        }
        spin_sleep::sleep(PLAY_MIN_INTERVAL * 10);
    }

    // loop {
    //     if ctrl.abort_request() {
    //         break;
    //     }

    //     if ctrl.pause() {
    //         spin_sleep::sleep(Duration::from_millis(50));
    //         continue;
    //     }

    //     if let Some((combine_frame, mut duration)) = play_queue.fetch_frame() {
    //         // 更新延时时间
    //         let elapsed = start.elapsed() - play_end_time;
    //         duration = if duration > elapsed {
    //             duration - elapsed
    //         } else {
    //             PLAY_MIN_INTERVAL
    //         };
    //         log::info!(
    //             "combine_frame.len(): {}, duration: {:?}, elapsed: {:?}",
    //             combine_frame.len(),
    //             &duration,
    //             &elapsed
    //         );
    //         if !play_frame(&ctrl, combine_frame, duration) {
    //             break;
    //         }
    //         play_end_time = start.elapsed();
    //     } else {
    //         log::info!("combine_frame is empty");
    //         if play_queue.mem_size() < PLAY_DATA_MEM_MAX {
    //             while let Ok(frame) = audio_frame_queue.try_recv() {
    //                 log::info!("{:?}", &frame);
    //                 play_queue.insert(PlayFrame::Audio(frame));
    //             }
    //             while let Ok(frame) = video_frame_queue.try_recv() {
    //                 log::info!("{:?}", &frame);
    //                 play_queue.insert(PlayFrame::Video(frame));
    //             }
    //             log::info!("play_queue.len(): {}", play_queue.len());
    //             if play_queue.is_empty() {
    //                 count += 1;
    //                 if count == 3 {
    //                     break;
    //                 }
    //                 spin_sleep::sleep(PLAY_MIN_INTERVAL);
    //             } else {
    //                 count = 0;
    //             }
    //         }
    //     }
    // }

    log::info!("play elapsed: {:?}", start.elapsed());
}

/// 返回值: false 退出, true 继续
fn play_frame(ctrl: &PlayControl, mut frame: PlayQueue, duration: Duration) -> bool {
    while let Some(frame) = frame.pop() {
        match frame {
            PlayFrame::Audio(audio) => {
                log::info!("{audio:?}");
                ctrl.play_audio(audio);
            }
            PlayFrame::Video(video) => {
                log::info!("{video:?}");
                match ctrl.send_state(PlayState::Video(video)) {
                    Ok(_) | Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => {
                        log::info!("play channel disconnected");
                        return false;
                    }
                }
            }
        }
    }
    spin_sleep::sleep(duration);
    true
}

#[derive(Default)]
pub struct PlayQueue {
    mem_size: usize,
    queue: LinkedList<PlayFrame>,
}

impl PlayQueue {
    pub fn new() -> Self {
        Self {
            mem_size: std::mem::size_of::<Self>(),
            queue: LinkedList::new(),
        }
    }

    pub fn mem_size(&self) -> usize {
        self.mem_size
    }

    pub fn fetch_frame(&mut self) -> Option<(Self, Duration)> {
        let mut cursor = self.queue.cursor_front();
        let mut idx = None;
        let mut duration = Duration::default();

        if let Some(first) = cursor.current() {
            let (start_pts, _) = first.time_info();
            cursor.move_next();
            while let Some(frame) = cursor.current() {
                let (pts, _) = frame.time_info();
                duration = Duration::from_secs_f64(pts - start_pts);
                if duration >= PLAY_MIN_INTERVAL && duration <= PLAY_MAX_INTERVAL {
                    log::info!("start_pts:{}, pts:{}", start_pts, pts,);
                    idx = cursor.index();
                    break;
                }
                cursor.move_next();
            }
        }
        if let Some(idx) = idx {
            let mut queue = Self::new();
            for _ in 0..idx {
                if let Some(frame) = self.pop() {
                    queue.push(frame);
                }
            }
            return Some((queue, duration));
        }
        None
    }

    pub fn pop(&mut self) -> Option<PlayFrame> {
        let frame = self.queue.pop_front();
        if let Some(frame) = frame.as_ref() {
            let mem_size = frame.mem_size();
            self.mem_size -= mem_size;
        }
        frame
    }

    pub fn push(&mut self, new_frame: PlayFrame) {
        let mem_size = new_frame.mem_size();
        self.queue.push_back(new_frame);
        self.mem_size += mem_size;
    }

    /// 添加一帧数据, 按照pts由小到大的顺序依次插入
    pub fn insert(&mut self, new_frame: PlayFrame) {
        let mem_size = new_frame.mem_size();
        let (pts, _) = new_frame.time_info();
        let mut new_frame = Some(new_frame);
        let mut cursor = self.queue.cursor_back_mut();
        while let Some(frame) = cursor.current() {
            let (pts_, _) = frame.time_info();
            if pts_ <= pts {
                cursor.insert_after(new_frame.take().unwrap());
                break;
            }
            cursor.move_prev();
        }
        if let Some(new_frame) = new_frame {
            cursor.insert_after(new_frame);
        }
        self.mem_size += mem_size;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }
}
