#[derive(Debug)]
pub enum PlayerEvent {
    /// 开始播放
    Start(String),
    /// 暂停
    Pause,
    /// 当前播放结束
    END,
    /// 停止
    Stop,
    /// 上一首
    Previous,
    /// 下一首
    Next,
    /// 打开目录
    OpenFolder,
    /// 静音
    Mute(bool),
    /// 调节音量
    Volume(u8),
    /// 当前视频信息(video index, filename)
    Current(u32, String),
    /// 应用离开
    Exit,
    /// 全屏
    Fullscreen,
    /// 最大化
    Maximize,
    /// 最小化
    Minimize,
    /// 拖拽窗口
    DragWindow,
}

/// 用于处理 Video 编解码, 及 视频控制
pub enum VideoEvent {
    /// 音频数据
    Audio(Vec<f32>),
    /// 视频数据
    Video(Vec<u32>),
}