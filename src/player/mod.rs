use anyhow::{Context, Result};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player as AudioPlayer, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

/// 持有音频设备和播放队列，并维护扣除暂停时间后的播放进度。
pub struct Player {
    sink: AudioPlayer,
    stream: MixerDeviceSink,
    /// 解码器提供的总时长；未知时由界面显示占位符。
    pub duration: Option<Duration>,
    elapsed: Duration,
    started: Option<Instant>,
    loaded: bool,
}

fn decode(path: &Path) -> Result<Decoder<BufReader<File>>> {
    let file = File::open(path).with_context(|| format!("无法打开：{}", path.display()))?;
    // 直接从文件构建解码器，提供 M4A 容器解析所需的文件长度和定位能力。
    Decoder::try_from(file).with_context(|| format!("无法解码音频：{}", path.display()))
}

impl Player {
    /// 打开默认音频输出设备；设备不可用时返回错误。
    pub fn new() -> Result<Self> {
        let mut stream = DeviceSinkBuilder::open_default_sink().context("无法打开音频输出设备")?;
        stream.log_on_drop(false);
        let sink = AudioPlayer::connect_new(stream.mixer());
        Ok(Self {
            sink,
            stream,
            duration: None,
            elapsed: Duration::ZERO,
            started: None,
            loaded: false,
        })
    }
    /// 加载并播放文件，以给定音量替换当前歌曲；解码失败时保留原播放。
    pub fn load(&mut self, path: &Path, volume: f32) -> Result<()> {
        // 先验证新音频，再停止原歌曲，避免加载失败造成不必要的中断。
        let source = decode(path)?;
        let duration = source.total_duration();
        let sink = AudioPlayer::connect_new(self.stream.mixer());
        sink.set_volume(volume);
        sink.pause();
        sink.append(source);
        self.sink.stop();
        self.sink = sink;
        self.duration = duration;
        self.elapsed = Duration::ZERO;
        self.started = Some(Instant::now());
        self.loaded = true;
        self.sink.play();
        Ok(())
    }
    /// 暂停或恢复当前歌曲；尚未加载或已经结束时不执行操作。
    pub fn toggle(&mut self) {
        if !self.loaded || self.sink.empty() {
            return;
        }
        if self.sink.is_paused() {
            self.started = Some(Instant::now());
            self.sink.play();
        } else {
            self.elapsed = self.position();
            self.started = None;
            self.sink.pause();
        }
    }
    /// 按运行时钟估算已播放时间，排除暂停时间并限制在已知总时长内。
    pub fn position(&self) -> Duration {
        let elapsed = self.elapsed
            + self
                .started
                .map(|start| start.elapsed())
                .unwrap_or_default();
        self.duration
            .map(|duration| elapsed.min(duration))
            .unwrap_or(elapsed)
    }
    /// 返回播放队列是否处于暂停状态。
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
    /// 仅在已加载歌曲且播放队列耗尽时报告播放结束。
    pub fn finished(&self) -> bool {
        self.loaded && self.sink.empty()
    }
    /// 设置音量，调用方将值限制在 0.0 至 1.0。
    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }
    /// 停止并清空播放队列，保留停止时的进度用于显示。
    pub fn stop(&mut self) {
        self.elapsed = self.position();
        self.started = None;
        self.loaded = false;
        self.sink.stop();
    }
}

#[cfg(test)]
/// 音频解码测试及供应用流程测试复用的音频生成工具。
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// 写入一秒静音 WAV，用于无需外部音频素材的测试。
    pub fn wav(path: &Path) {
        // 使用 8 kHz、16 位单声道 PCM，并写入对应的 WAV 文件头。
        let mut file = File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&16036u32.to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.write_all(&16000u32.to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&16000u32.to_le_bytes()).unwrap();
        file.write_all(&vec![0; 16000]).unwrap();
    }
    #[test]
    fn decode_wav_and_reject_invalid_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("歌曲.wav");
        wav(&path);
        let source = decode(&path).unwrap();
        assert_eq!(source.total_duration(), Some(Duration::from_secs(1)));
        assert_eq!(source.count(), 8000);
        std::fs::write(&path, b"not audio").unwrap();
        assert!(decode(&path).is_err());
        assert!(decode(&temp.path().join("missing.wav")).is_err());
    }
    #[test]
    #[ignore = "requires an audio output device"]
    fn audio_device_play_pause_switch_and_finish() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.wav");
        let second = temp.path().join("second.wav");
        wav(&first);
        wav(&second);
        let mut player = Player::new().unwrap();
        player.load(&first, 0.0).unwrap();
        assert!(!player.finished());
        player.toggle();
        assert!(player.is_paused());
        let position = player.position();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(player.position(), position);
        player.toggle();
        assert!(!player.is_paused());
        player.load(&second, 0.0).unwrap();
        assert!(player.position() < Duration::from_millis(100));
        assert!(player.load(&temp.path().join("missing.wav"), 0.0).is_err());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !player.finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(player.finished());
    }
}
