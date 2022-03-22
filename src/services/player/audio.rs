use std::ffi::CString;
use std::time::{Duration, Instant};

use cpal::traits::HostTrait;
use cpal::SupportedStreamConfig;
use crossbeam_channel::Sender;
use rodio::buffer::SamplesBuffer;
use rodio::{DeviceTrait, OutputStream, Sample, Sink, Source};
use rsmpeg::avfilter::AVFilterGraph;

use crate::error::{PlayerError, Result};
use crate::services::player::stream::{decode_frame, DecodeContext};
use crate::services::player::PlayState;

pub fn audio_decode_thread(mut decode_ctx: DecodeContext, state_tx: Sender<PlayState>) {
    let audio_dev = AudioDevice::new()
        .map_err(|e| {
            state_tx.send(PlayState::Error(e)).ok();
        })
        .unwrap();

    let mut audio_graph = None;

    let start = Instant::now();

    let mut duration = Duration::default();

    let mut first_time = Option::<Duration>::None;
    let mut pts = Duration::default();

    loop {
        let source = fetch_audio_source(&mut decode_ctx, &mut audio_graph, &audio_dev);
        let source = match source {
            Ok(None) => break,
            Ok(Some((source, _pts))) => {
                pts = _pts;
                source
            }
            Err(e) => {
                log::error!("{}", e.to_string());
                break;
            }
        };
        // 取出一帧播放, 预取下一帧后, 再根据 pts(显示时间) 和 duration(间隔), 休眠一定的时间
        // 预取操作可以让当前帧播放结束后, 下一帧立即播放, 时间误差会极小
        loop {
            if first_time.is_none() {
                duration = source.total_duration().unwrap();
                first_time = Some(start.elapsed());
                log::info!(
                    "audio pts: {pts:?}, duration: {:?}ms",
                    &duration.as_millis()
                );
                audio_dev.play_source(source, decode_ctx.volume());
                break;
            } else {
                let duration = first_time.take().unwrap() + duration - start.elapsed();
                spin_sleep::sleep(duration);
            }
        }
    }
    log::info!("audio解码线程退出, elapsed: {:?}", start.elapsed());
}

pub fn fetch_audio_source(
    decode_ctx: &mut DecodeContext,
    audio_graph: &mut Option<AVFilterGraph>,
    audio_dev: &AudioDevice,
) -> Result<Option<(SamplesBuffer<f32>, Duration)>> {
    let raw_frame = match decode_frame(decode_ctx) {
        Ok(None) => {
            log::info!("audio decode exited");
            return Ok(None);
        }
        Ok(Some(frame)) => frame,
        Err(e) => {
            return Err(PlayerError::Error(e.to_string()));
        }
    };

    log::debug!(
        "audio queue mem_size: {} MByte",
        decode_ctx.queue_mem_size() as f32 / 1000000.0
    );

    if audio_graph.is_none() {
        let default_config = audio_dev.default_config_ref();
        *audio_graph = Some(
            audio_graph_parse(
                raw_frame.sample_rate,
                raw_frame.format,
                raw_frame.channel_layout,
                raw_frame.channels,
                default_config.sample_rate().0,
            )
            .expect("Error while audio_graph_parse"),
        );
    }

    let graph = audio_graph.as_mut().unwrap();

    graph
        .get_filter(cstr::cstr!("abuffer@audio0"))
        .expect("get abuffer@audio0 failed")
        .buffersrc_add_frame(Some(raw_frame), None)
        .expect("Error while feeding the filtergraph");

    let ctx = &mut graph
        .get_filter(cstr::cstr!("abuffersink@out"))
        .expect("get abuffersink@out failed");

    let frame = ctx
        .buffersink_get_frame(None)
        .expect("Get frame from buffer sink failed");

    let time_base = 1.0 / frame.sample_rate as f64;

    let pts = Duration::from_secs_f64(time_base * frame.pts as f64);

    let data_len = frame.nb_samples as usize * frame.channels as usize;

    let samples = unsafe {
        #[allow(clippy::cast_ptr_alignment)]
        std::slice::from_raw_parts(frame.data[0] as *const i32, data_len)
    };
    let samples: Vec<f32> = samples
        .iter()
        .map(|v| (*v as f32) / i32::MAX as f32)
        .collect();

    let source = SamplesBuffer::new(frame.channels as u16, frame.sample_rate as u32, samples);

    Ok(Some((source, pts)))
}

// AudioDevice::SAMPLE_RATE.0
pub fn audio_graph_parse(
    src_sample_rate: i32,
    src_format: i32,
    src_channel_layout: u64,
    src_channels: i32,
    dst_sample_rate: u32,
) -> Result<AVFilterGraph> {
    // 上下两部分, 上面是高清原始屏, 下面是低分辨率的 机械屏/龙鳞屏
    // 一个 视频源文件的帧 和 发送到screen上的 RGB帧, 合并

    // 创建 Graph
    let filter_graph = AVFilterGraph::new();

    // 构建 filter spec
    let buffer0_filter = format!(
        "abuffer@audio0=sample_rate={}:sample_fmt={}:channels={}:channel_layout={} [audio0_src]",
        src_sample_rate, src_format, src_channels, src_channel_layout
    );

    let format_filter = format!(
        "[audio0_src] aformat=sample_rates={}:sample_fmts=s32:channel_layouts=stereo [audio0_out]",
        dst_sample_rate
    );

    let buffersink_filter = "[audio0_out] abuffersink@out";

    let filter_spec = &CString::new(format!(
        "{}; {}; {}",
        buffer0_filter, format_filter, buffersink_filter
    ))?;

    log::debug!("filter_spec: {:?}", filter_spec);

    // 解析 filter spec
    filter_graph.parse_ptr(filter_spec, None, None)?;

    filter_graph.config()?;

    Ok(filter_graph)
}

pub struct AudioDevice {
    _stream: OutputStream,
    sink: Sink,
    default_config: SupportedStreamConfig,
}

impl AudioDevice {
    pub fn new() -> Result<Self> {
        let default_device = cpal::default_host()
            .default_output_device()
            .ok_or(PlayerError::NoAudioDevice)?;

        let default_config = default_device
            .default_output_config()
            .map_err(|e| PlayerError::DefaultAudioStreamConfigError(e.to_string()))?;

        let default_stream = OutputStream::try_from_device(&default_device);

        let (_stream, handle) = default_stream
            .or_else(|original_err| {
                // default device didn't work, try other ones
                let mut devices = match cpal::default_host().output_devices() {
                    Ok(d) => d,
                    Err(_) => return Err(original_err),
                };
                devices
                    .find_map(|d| OutputStream::try_from_device(&d).ok())
                    .ok_or(original_err)
            })
            .map_err(|e| PlayerError::CreateAudioStreamError(e.to_string()))?;

        let sink = rodio::Sink::try_new(&handle).unwrap();
        Ok(Self {
            _stream,
            sink,
            default_config,
        })
    }

    pub fn default_config_ref(&self) -> &SupportedStreamConfig {
        &self.default_config
    }

    pub fn play_source<S>(&self, audio_source: S, volume: f32)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
    {
        self.sink.append(audio_source);
        self.sink.set_speed(volume);
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn stop(&self) {
        self.sink.stop();
    }

    pub fn set_volume(&self, value: f32) {
        self.sink.set_volume(value);
    }
}

// #[derive(Clone)]
// pub struct AudioSource {
//     pub samples: IntoIter<f32>,
//     pub duration: Duration,
//     pub channels: u16,
//     pub sample_rate: u32,
// }

// impl AudioSource {
//     pub fn from(samples: Vec<f32>, duration: Duration, channels: u16, sample_rate: u32) -> Self {
//         Self {
//             samples: samples.into_iter(),
//             channels,
//             duration,
//             sample_rate,
//         }
//     }
// }

// impl std::fmt::Debug for AudioSource {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("AudioSource")
//             .field("samples len", &self.samples.len())
//             .field("duration", &self.duration)
//             .field("channels", &self.channels)
//             .field("sample_rate", &self.sample_rate)
//             .finish()
//     }
// }

// impl Iterator for AudioSource {
//     type Item = f32;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.next()
//     }
// }

// impl Source for AudioSource {
//     fn current_frame_len(&self) -> Option<usize> {
//         Some(self.samples.len())
//     }

//     fn channels(&self) -> u16 {
//         self.channels
//     }

//     fn sample_rate(&self) -> u32 {
//         self.sample_rate
//     }

//     fn total_duration(&self) -> Option<std::time::Duration> {
//         Some(self.duration)
//     }
// }
