use std::ffi::CString;
use std::time::Duration;
use std::vec::IntoIter;

use cpal::traits::HostTrait;
use cpal::SupportedStreamConfig;
use rodio::{DeviceTrait, OutputStream, Sample, Sink, Source};
use rsmpeg::avfilter::AVFilterGraph;

use crate::error::{PlayerError, Result};
use crate::services::player::stream::{decode_frame, DecodeContext};

pub fn audio_decode_thread(mut decode_ctx: DecodeContext) {
    let mut audio_graph = None;

    loop {
        let source = fetch_audio_source(&mut decode_ctx, &mut audio_graph);
        let audio = match source {
            Ok(None) => break,
            Ok(Some(source)) => source,
            Err(e) => {
                log::error!("{}", e.to_string());
                break;
            }
        };
        if let Err(_) = decode_ctx.send_audio(audio) {
            log::info!("audio thread disconnected");
            break;
        }
    }
    log::info!("audio 解码线程退出");
}

pub fn fetch_audio_source(
    decode_ctx: &mut DecodeContext,
    audio_graph: &mut Option<AVFilterGraph>,
) -> Result<Option<AudioFrame>> {
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
        let default_config = decode_ctx.audio_default_config();
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

    let data_len = frame.nb_samples as usize * frame.channels as usize;

    // 音频的时间基 就是一个采样的时间, 即 采样率的倒数
    let time_base = 1.0 / frame.sample_rate as f64;

    let pts = time_base * frame.pts as f64;
    let duration = time_base * data_len as f64;

    let volume = decode_ctx.volume();
    let samples = unsafe { std::slice::from_raw_parts(frame.data[0] as *const f32, data_len) };
    let samples: Vec<f32> = samples.iter().map(|s| s * volume).collect();

    let source = AudioFrame::new(
        samples,
        frame.channels as u16,
        frame.sample_rate as u32,
        pts,
        duration,
    );

    Ok(Some(source))
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
        "[audio0_src] aformat=sample_rates={}:sample_fmts=flt:channel_layouts=stereo [audio0_out]",
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

    pub fn default_config(&self) -> SupportedStreamConfig {
        self.default_config.clone()
    }

    pub fn play_source<S>(&self, audio_source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
    {
        self.sink.append(audio_source);
    }

    pub fn set_mute(&self, mute: bool) {
        if mute {
            self.sink.set_volume(0.0);
        } else {
            self.sink.set_volume(1.0);
        }
    }

    pub fn set_pause(&self, pause: bool) {
        if pause {
            self.sink.pause();
        } else {
            self.sink.play();
        }
    }

    pub fn stop(&self) {
        self.sink.stop();
    }
}

unsafe impl Send for AudioDevice {}
unsafe impl Sync for AudioDevice {}

#[derive(Clone)]
pub struct AudioFrame {
    pub samples: IntoIter<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub pts: f64,
    pub duration: f64,
}

impl AudioFrame {
    pub fn new(
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
        pts: f64,
        duration: f64,
    ) -> Self {
        Self {
            samples: samples.into_iter(),
            channels,
            sample_rate,
            pts,
            duration,
        }
    }
}

impl std::fmt::Debug for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFrame")
            .field("samples len", &self.samples.len())
            .field("channels", &self.channels)
            .field("sample_rate", &self.sample_rate)
            .field("pts", &self.pts)
            .field("duration", &self.duration)
            .finish()
    }
}

impl Iterator for AudioFrame {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.samples.next()
    }
}

impl Source for AudioFrame {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len())
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(Duration::from_secs_f64(self.duration))
    }
}
