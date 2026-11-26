//! 独立保存运行状态，不修改用户手写的配置文件。
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 自然播放结束后的续播方式；手动切歌不受此模式影响。
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    /// 按顺序播放，到列表末尾停止。
    #[default]
    Sequential,
    /// 播完列表后回到第一首。
    RepeatAll,
    /// 重新播放当前歌曲。
    RepeatOne,
}

impl PlaybackMode {
    /// 按顺序播放、列表循环、单曲循环的顺序切换。
    pub fn next(self) -> Self {
        match self {
            Self::Sequential => Self::RepeatAll,
            Self::RepeatAll => Self::RepeatOne,
            Self::RepeatOne => Self::Sequential,
        }
    }

    /// 返回界面显示的中文模式名称。
    pub fn label(self) -> &'static str {
        match self {
            Self::Sequential => "顺序播放",
            Self::RepeatAll => "列表循环",
            Self::RepeatOne => "单曲循环",
        }
    }
}

/// 可跨启动恢复的状态；曲目保存路径而非易随扫描变化的索引。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct SessionState {
    pub music_directory: Option<PathBuf>,
    pub current_track: Option<PathBuf>,
    /// 线性音量，合法范围为 0.0 至 1.0。
    pub volume: f32,
    pub playback_mode: PlaybackMode,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            music_directory: None,
            current_track: None,
            volume: 1.0,
            playback_mode: PlaybackMode::default(),
        }
    }
}

impl SessionState {
    /// 缺少文件时使用默认状态，其他读取或内容错误交由启动流程提示。
    pub fn load(path: &Path) -> Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error).context("无法读取播放状态"),
        };
        let state: Self = toml::from_str(&text).context("无效的播放状态文件")?;
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.volume.is_finite() && (0.0..=1.0).contains(&self.volume),
            "状态音量必须为 0 至 1"
        );
        Ok(())
    }

    /// 在目标目录写入临时文件并替换，写入失败时保留原有状态文件。
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let text = toml::to_string_pretty(self).context("无法序列化播放状态")?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::Builder::new()
            .prefix(".state-")
            .tempfile_in(parent)?;
        temporary.write_all(text.as_bytes())?;
        temporary.as_file().sync_all()?;
        temporary
            .persist(path)
            .map_err(|error| error.error)
            .with_context(|| format!("无法保存播放状态：{}", path.display()))?;
        Ok(())
    }
}

/// 目录、曲目和模式变化立即保存，连续音量变化合并为最后一次变化后半秒保存。
pub struct StateWriter {
    path: PathBuf,
    observed: Option<SessionState>,
    pending_since: Option<Instant>,
}

impl StateWriter {
    /// 创建与当前工作目录状态文件绑定的写入器，首次同步会保存目录。
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            observed: None,
            pending_since: None,
        }
    }

    /// 同步最新状态；退出时强制保存，失败后等待新变化或退出再重试。
    pub fn sync(&mut self, state: SessionState, now: Instant, force: bool) -> Result<()> {
        let immediate = self.observed.as_ref().is_none_or(|old| {
            old.music_directory != state.music_directory
                || old.current_track != state.current_track
                || old.playback_mode != state.playback_mode
        });
        if self.observed.as_ref() != Some(&state) {
            self.pending_since = Some(now);
            self.observed = Some(state.clone());
        }
        let due = self.pending_since.is_some_and(|since| {
            now.saturating_duration_since(since) >= Duration::from_millis(500)
        });
        if force || immediate || due {
            self.pending_since = None;
            state.save(&self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_defaults_roundtrips_and_saves_immediately() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.toml");
        fs::write(&path, "volume = 0.4").unwrap();
        let mut state = SessionState::load(&path).unwrap();
        assert_eq!(state.playback_mode, PlaybackMode::Sequential);
        let mut writer = StateWriter::new(path.clone());
        let now = Instant::now();
        writer.sync(state.clone(), now, false).unwrap();
        for mode in [
            PlaybackMode::RepeatAll,
            PlaybackMode::RepeatOne,
            PlaybackMode::Sequential,
        ] {
            state.playback_mode = mode;
            writer.sync(state.clone(), now, false).unwrap();
            assert_eq!(SessionState::load(&path).unwrap(), state);
        }
        fs::write(&path, "playback_mode = 'invalid'").unwrap();
        assert!(SessionState::load(&path).is_err());
    }

    #[test]
    fn roundtrip_replace_and_invalid_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.toml");
        assert_eq!(SessionState::load(&path).unwrap(), SessionState::default());
        let mut state = SessionState {
            music_directory: Some(temp.path().into()),
            current_track: Some(temp.path().join("专辑/歌曲.m4a")),
            volume: 0.6,
            playback_mode: PlaybackMode::RepeatOne,
        };
        state.save(&path).unwrap();
        assert_eq!(SessionState::load(&path).unwrap(), state);
        state.volume = 0.2;
        state.save(&path).unwrap();
        assert_eq!(SessionState::load(&path).unwrap(), state);
        let before = fs::read(&path).unwrap();
        state.volume = f32::NAN;
        assert!(state.save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        for content in ["[broken", "volume = nan", "volume = 2.0"] {
            fs::write(&path, content).unwrap();
            assert!(SessionState::load(&path).is_err());
        }
    }

    #[test]
    fn debounce_track_change_and_exit_flush() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.toml");
        let mut writer = StateWriter::new(path.clone());
        let mut state = SessionState::default();
        let now = Instant::now();
        writer.sync(state.clone(), now, false).unwrap();
        state.volume = 0.5;
        writer.sync(state.clone(), now, false).unwrap();
        state.volume = 0.3;
        writer
            .sync(state.clone(), now + Duration::from_millis(300), false)
            .unwrap();
        writer
            .sync(state.clone(), now + Duration::from_millis(700), false)
            .unwrap();
        assert_eq!(SessionState::load(&path).unwrap().volume, 1.0);
        writer
            .sync(state.clone(), now + Duration::from_millis(800), false)
            .unwrap();
        assert_eq!(SessionState::load(&path).unwrap().volume, 0.3);
        state.current_track = Some(temp.path().join("song.wav"));
        writer
            .sync(state.clone(), now + Duration::from_millis(810), false)
            .unwrap();
        assert_eq!(SessionState::load(&path).unwrap(), state);
        state.volume = 0.7;
        writer
            .sync(state.clone(), now + Duration::from_millis(820), true)
            .unwrap();
        assert_eq!(SessionState::load(&path).unwrap(), state);
    }
}
