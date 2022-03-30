#[derive(Debug)]
pub enum PlayerEvent {
    /// 打开文件
    OpenFile,

    /*
        窗口
    */
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

    /*
        播放控制
    */
    /// 终止
    Terminate,
    /// 暂停
    Pause(bool),
    /// 上一首
    Previous,
    /// 开始播放, 播放选中的或者第一个文件
    Play,
    /// 下一首
    Next,
    /// 循环
    Loop,
    /// 静音
    Mute(bool),
    /// 调节音量
    Volume(f32),

    /// 当前视频信息(video index, filename)
    Current(u32, String),
}
