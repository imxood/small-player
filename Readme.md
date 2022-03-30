这是一个简单的FFMPEG播放器, 使用 bevy 和 egui 构建的 ui界面.

学习与实践是目的.

如果喜欢这个项目, 请 star 或 fork.

![](docs/Readme/2022-03-30-22-24-17.png)

当前已实现:
视频文件的 播放/暂停/停止/上一首/下一首/音量控制

存在问题:
依然有一些内存无法释放, 应该是FFMPEG的内存, 原因暂时不明
启动后, 没有播放时内存是31Mb, 每播放一个视频, 内存会增加一点, 循环播放3个视频 平均每个视频时长1分钟, 最高清为1080P, 播放2个小时后会占用大概592Mb内存

## 编译环境

### 安装FFMPEG

#### windows 10

需要安装 vcpkg:

    打开 powershell, 执行:

    git clone https://github.com/Microsoft/vcpkg.git
    cd vcpkg
    .\bootstrap-vcpkg.bat -disableMetrics
    .\vcpkg.exe integrate install

使用 vcpkg 编译 (我开了代理, 总共编译大概36分钟):

    .\vcpkg.exe install ffmpeg[ffmpeg,ffplay,ffprobe,x265,avcodec,avdevice,fdk-aac] --triplet x64-windows-static-md --recurse

#### ubuntu

```sh
git clone https://github.com/ffmpeg/ffmpeg.git

cd ffmpeg

make clean

./configure --prefix="/develop/programs/ffmpeg_build" --ld="clang" --enable-pic --disable-programs --disable-avdevice --disable-postproc --disable-network --disable-schannel --disable-sdl2 --disable-sndio

make -j
make install

# 编译 ffmpeg 的例子
make examples -j
```

### 编译并运行

```sh
# 使用 nightly 编译
rustup default nightly

# 如果是 debug 版本, 在执行 解码后的Rgb数据 转 egui的 Color Image时 会特别慢, release版本会有优化

cargo run --release

# 设置环境变量 "WGPU_BACKEND=.." 可以给wgpu选择不同的后端, 如: WGPU_BACKEND=gl, 使用opengl.
```
## 学习记录

### 音视频同步

非常重要的参考:
* [音视频同步](https://www.cnblogs.com/leisure_chn/p/10307089.html)
* [播放器技术分享（3）：音画同步](https://zhuanlan.zhihu.com/p/51924640)

对于 一个44.1KHz的AAC音频流 (一秒声音中有44.1K个数据点), 每个声道可能包含1024个采样点, 即: 一帧声音中 需要采样 1024个数据, 那这一帧的时间就是
平均每个数据点的时间 乘以 采样数, 在乘以1000, 得到一帧音频数据的毫秒时间, 即: 1/44.1K * 1024 * 1000ms ≈ 23.22ms

#### time_base 时间基

    time_base是PTS和DTS的时间单位，也称时间基

    不同的封装(mp4 flv等) time_base也不相同

    从流中可以获取到时间基, 这表示基本单位, 用于后面 显示时间的计算
    AVRational tb = is->p_video_stream->time_base;

    当前帧的帧速率, 表示 1秒多少帧
    AVRational frame_rate = av_guess_frame_rate(is->p_fmt_ctx, is->p_video_stream, NULL);

    // 根据帧速率 算一下 两帧时间间隔, 即 当前帧播放时长
    duration = (frame_rate.num && frame_rate.den ? av_q2d((AVRational){frame_rate.den, frame_rate.num}) : 0);
    
    // 计算显示时间, 即 当前帧显示时间戳
    pts = (p_frame->pts == AV_NOPTS_VALUE) ? NAN : p_frame->pts * av_q2d(tb);

### AV_DISPOSITION_ATTACHED_PIC

对于mp3文件 判断流中是否绑定了图片

    av_stream.disposition & AV_DISPOSITION_ATTACHED_PIC

对于一个有封面的mp3文件, 它可以包含 audio stream 和 video stream, 它的 video stream 中只包含了一个包, 那么就可以通过 video_stream.disposition 判断这个流中保存了一个绑定图片

学习自: [FFmpeg小点记】AV_DISPOSITION_ATTACHED_PIC](https://segmentfault.com/a/1190000018373504)

## ffmpeg 常用命令
    
```sh
# 显示音视频文件的Packages信息:
ffprobe -show_packets -of json -i quliulang.mp3 > packets.json

# 显示所有可用的封装
ffmpeg -formats
```

## 音频

[PCM音频采样数据处理](https://blog.csdn.net/leixiaohua1020/article/details/50534316)

## FFMPEG 内存管理

参考 [FFmpeg视频播放的内存管理](https://www.jianshu.com/p/9f45d283d904)

av_frame_alloc
    只是给AVFrame分配了内存，它内部的buf还是空的，就相当于造了一个箱子，但箱子里是空的。

av_frame_ref
    对src的buf增加一个引用，即使用同一个数据，只是这个数据引用计数+1.av_frame_unref把自身对buf的引用释放掉，数据的引用计-1。

av_frame_free
    内部还是调用了unref,只是把传入的frame也置空