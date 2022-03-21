#[derive(Debug)]
pub enum PlayerEvent {
    /// 打开目录
    OpenFolder,
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
    /// 开始播放
    Start(String),
    /// 停止
    Stop,
}

pub enum PlayEvent {
    /// 暂停
    Pause,
    /// 当前播放结束
    END,
    /// 上一首
    Previous,
    /// 下一首
    Next,
    /// 静音
    Mute(bool),
    /// 调节音量
    Volume(f32),
    /// 当前视频信息(video index, filename)
    Current(u32, String),
}
