use std::{
    collections::LinkedList,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, TrySendError};

use crate::defines::PLAY_MIN_INTERVAL;

use super::{audio::AudioFrame, PlayControl, PlayState, VideoFrame};

#[derive(Debug)]
pub enum PlayFrame {
    Audio(AudioFrame),
    Video(VideoFrame),
}

impl PlayFrame {
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
    let mut play_queue = PlayQueue::new();
    let mut count = 0;

    let start = Instant::now();

    loop {
        let before_point = start.elapsed();
        if ctrl.abort_request() {
            break;
        }

        if ctrl.pause() {
            spin_sleep::sleep(Duration::from_millis(50));
            continue;
        }

        while let Ok(frame) = audio_frame_queue.try_recv() {
            play_queue.insert(PlayFrame::Audio(frame));
        }

        while let Ok(frame) = video_frame_queue.try_recv() {
            play_queue.insert(PlayFrame::Video(frame));
        }

        let mut audios = Vec::new();
        let mut video = Option::<VideoFrame>::None;

        let mut duration = 0.0;
        let mut first_pts = None;
        // 所有帧的后续的一帧的pts
        // 这个pts 减去 first_pts 就等于 当前线程需要休眠的时间, 用于等待 预取数据播放结束
        let mut end_pts = 0.0;

        // 预取最多 PLAY_MIN_INTERVAL 时间的数据, 如果 已经取到一个视频, 那么 再取到一个视频时, 也会结束取数据
        while let Some(frame0) = play_queue.pop() {
            let (pts, _duration) = frame0.time_info();
            // log::debug!("pts: {pts}, duration: {duration}");
            if first_pts.is_none() {
                first_pts = Some(pts);
            }
            match frame0 {
                PlayFrame::Audio(audio) => {
                    audios.push(audio);
                }
                PlayFrame::Video(video_) => {
                    video = Some(video_);
                }
            };
            end_pts = pts;

            let frame1 = play_queue.peek();
            if let Some(frame) = frame1 {
                let (pts, _duration) = frame.time_info();
                // 如果要播放这一帧 将超出播放间隔, 那么就不播放这一帧, 结束取帧
                if pts - *first_pts.as_ref().unwrap() > PLAY_MIN_INTERVAL {
                    end_pts = pts;
                    // log::info!("pts: {pts}, duration: {duration}");
                    break;
                }
                // 如果再次遇到一个视频, 则结束取帧
                match frame {
                    PlayFrame::Audio(_) => continue,
                    PlayFrame::Video(_) => {
                        end_pts = pts;
                        // log::info!("pts: {pts}, duration: {duration}");
                        if video.is_some() {
                            break;
                        }
                    }
                }
            }
        }

        if video.is_none() && audios.is_empty() {
            count += 1;
            if count == 3 {
                ctrl.set_playing(false);
            }
            duration = 0.05;
        } else {
            duration = end_pts - first_pts.unwrap();
            count = 0;
            ctrl.set_playing(true);
        }

        // log::info!("video:{} audios:{}", video.is_some(), audios.len());

        // 播放视频
        if let Some(video) = video {
            match ctrl.send_state(PlayState::Video(video)) {
                Ok(_) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Disconnected(_)) => {
                    log::info!("play channel disconnected");
                    return;
                }
            }
        }

        // 播放音频
        for audio in audios {
            ctrl.play_audio(audio);
        }

        let mut duration = Duration::from_secs_f64(duration);

        // elapsed: 当前时间 - 处理队列消耗的时间
        let elapsed = start.elapsed() - before_point;

        duration = if duration > elapsed {
            duration - elapsed
        } else {
            duration
        };

        log::debug!("duration: {:?}, elapsed: {:?}", &duration, &elapsed);
        spin_sleep::sleep(duration);
    }
    log::info!("play elapsed: {:?}", start.elapsed());
}

pub struct PlayQueue {
    queue: LinkedList<PlayFrame>,
}

impl PlayQueue {
    pub fn new() -> Self {
        Self {
            queue: LinkedList::new(),
        }
    }

    pub fn pop(&mut self) -> Option<PlayFrame> {
        self.queue.pop_front()
    }

    pub fn peek(&mut self) -> Option<&PlayFrame> {
        self.queue.front()
    }

    /// 添加一帧数据, 按照pts由小到大的顺序依次插入
    pub fn insert(&mut self, new_frame: PlayFrame) {
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
    }
}
