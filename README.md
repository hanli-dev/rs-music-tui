# rs-music-tui

一个离线、本地音乐终端播放器，界面包含顶部搜索、左侧音乐库、歌曲列表和底部播放条。

## 构建与运行

安装 Rust 和平台编译工具后：

```powershell
cargo build --release --locked
.\target\release\rs-music-tui.exe --dir "D:\Music"
```

Linux / macOS：

```sh
cargo build --release --locked
./target/release/rs-music-tui --dir ~/Music
```

Windows 需要 MSVC 构建工具；Linux 音频后端需要 ALSA 开发库及 pkg-config
（例如 Debian/Ubuntu 的 `libasound2-dev`、`pkg-config`）。
播放需要可用的系统音频输出设备。

- `-d, --dir <DIRECTORY>`：递归扫描目录；省略时使用保存的音乐目录，没有记录时使用当前工作目录。
- `--list`：只列出扫描到的音频路径，不打开终端界面或音频设备。
- `-c, --config <FILE>`：使用指定 TOML 配置。
- `--help`：显示命令行帮助。

支持 MP3、FLAC、WAV、OGG/Vorbis、M4A（AAC-LC / ALAC 苹果无损；不支持 DRM 加密文件），扩展名不区分大小写，跳过符号链接。
文件按路径排序，暂以文件名作为歌曲标题；目录列显示相对目录。
扫描按扩展名筛选，实际播放时校验音频；损坏或无法读取的文件会显示错误。
无法读取的子目录会跳过并提示，根目录无效则直接报错。

## 操作

| 按键 | 功能 |
| --- | --- |
| ↑ / ↓ 或 k / j | 选择歌曲 |
| Enter | 播放选中歌曲 |
| Space | 暂停 / 继续 |
| n / p | 下一首 / 上一首 |
| m | 循环切换顺序播放、列表循环、单曲循环 |
| - / = | 降低 / 提高音量 |
| / | 按文件名或目录筛选 |
| Enter / Esc（搜索时） | 结束输入 / 清除筛选 |
| PageUp / PageDown 或 Ctrl-u / Ctrl-d | 上下移动 10 行 |
| Home / End 或 Ctrl-a / Ctrl-e | 跳到列表首尾 |
| r | 重新扫描目录 |
| ? | 帮助 |
| q / Ctrl-c | 退出（帮助中 q 先关闭帮助） |

切歌以完整音乐库的路径顺序为准，筛选只影响显示和选曲。
手动切歌在首尾循环，不受播放模式影响。底部显示当前模式：

| 模式 | 歌曲结束后的行为 |
| --- | --- |
| 顺序播放（默认） | 播放下一首，最后一首后停止 |
| 列表循环 | 播放下一首，最后一首后回到第一首 |
| 单曲循环 | 重播当前歌曲 |

播放失败会显示错误并等待操作，不会无限重试。
不支持获取时长的文件显示 `--:--`，不影响播放。

## 配置

默认读取**当前工作目录**下的 `config.toml`（不存在时使用默认值）。
工作目录是启动命令所在的目录，不是可执行文件或 `--dir` 指定的音乐目录。
`--config` 可指定其他文件；显式指定的文件不存在或配置无效时会报错。
所有字段均可省略，省略的字段使用默认值。
支持交互行为、快捷键和主题配置，未识别的配置项会被忽略。
`reset_play` 快捷键用于重新扫描目录。

```toml
[behavior]
volume_increment = 10
tick_rate_milliseconds = 250

[keybindings]
next_track = "n"
previous_track = "p"
cycle_playback_mode = "m"

[theme]
selected = "LightBlue"
```

## 播放状态

程序自动将运行状态保存到当前工作目录的 `state.toml`，与手动编辑的 `config.toml` 分开。
不同工作目录使用各自的状态文件，`--config` 不改变状态文件位置。

```toml
music_directory = 'D:\Music'
current_track = 'D:\Music\专辑\歌曲.m4a'
volume = 0.6
playback_mode = "sequential"
```

- 启动目录优先级：`--dir`、保存的音乐目录、当前工作目录。
- 恢复上次曲目的选中位置、音量和播放模式，等待手动播放；不保存或恢复播放进度。
- `playback_mode` 支持 `sequential`（顺序播放）、`repeat_all`（列表循环）、`repeat_one`（单曲循环）；省略时使用顺序播放。
- 曲目按完整路径识别；文件不存在时选中第一首。保存目录不可用时提示并回退到工作目录，显式指定的目录无效则报错。
- 首次运行使用默认音量；状态文件损坏或无法读取时提示并使用默认状态。
- 进入播放器、切换模式、切歌或重新扫描导致当前曲目变化时保存；音量变化停止约半秒后保存，退出时再保存一次。
- 保存采用同目录临时文件替换；保存失败会提示，不中断播放。强制结束程序可能丢失尚未保存的音量调整。
- `--list` 可以读取状态选择目录，但不会创建或修改状态文件。

## 验证

```sh
cargo test --locked
cargo build --release --locked
# 需要实际音频输出设备；测试使用静音 WAV：
cargo test --locked -- --ignored --test-threads=1
```

音频播放使用 rodio，MP3 和 M4A 使用 Symphonia 解码，无需安装 FFmpeg。
当前是本地播放最小版本，暂不提供标签编辑、本地歌词、持久化歌单或快进。
