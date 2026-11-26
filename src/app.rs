use crate::config::user_config::UserConfig;
use crate::event::Key;
use crate::library::{self, Track};
use crate::player::Player;
use crate::state::{PlaybackMode, SessionState};
use anyhow::Result;
use ratatui::widgets::TableState;
use std::path::PathBuf;

/// 管理音乐库、界面选择和播放状态；界面行号与音乐库索引分开保存。
pub struct App {
    pub root: PathBuf,
    pub tracks: Vec<Track>,
    pub warnings: Vec<String>,
    /// 筛选结果中的每一行对应完整音乐库中的索引。
    pub visible: Vec<usize>,
    pub table: TableState,
    pub query: String,
    pub searching: bool,
    pub help: bool,
    /// 当前歌曲在完整音乐库中的索引，不随筛选结果改变。
    pub current: Option<usize>,
    /// 是否存在尚未结束的播放任务；暂停时仍为真。
    pub active: bool,
    /// 首次播放时才初始化音频设备，扫描和列出文件不需要设备。
    pub player: Option<Player>,
    pub volume: f32,
    /// 控制自然播放结束后的续播行为。
    pub playback_mode: PlaybackMode,
    pub message: String,
    pub user_config: UserConfig,
}

impl App {
    /// 扫描指定目录并建立初始选择，扫描失败时返回错误。
    pub fn new(directory: PathBuf, user_config: UserConfig) -> Result<Self> {
        let library = library::scan(&directory)?;
        let mut app = Self {
            root: library.root,
            tracks: library.tracks,
            warnings: library.warnings,
            visible: Vec::new(),
            table: TableState::default(),
            query: String::new(),
            searching: false,
            help: false,
            current: None,
            active: false,
            player: None,
            volume: 1.0,
            playback_mode: PlaybackMode::default(),
            message: String::new(),
            user_config,
        };
        app.filter();
        app.scan_message();
        Ok(app)
    }

    /// 恢复曲目定位、音量和模式，不打开音频设备，也不自动开始播放。
    pub fn restore(&mut self, state: &SessionState) {
        self.volume = state.volume;
        self.playback_mode = state.playback_mode;
        self.current = state
            .current_track
            .as_ref()
            .and_then(|path| self.tracks.iter().position(|track| &track.path == path));
        if let Some(index) = self.current {
            self.table.select(Some(index));
        }
    }

    /// 提取持久化快照；移动列表选择不会替换最后实际播放的曲目。
    pub fn session_state(&self) -> SessionState {
        SessionState {
            music_directory: Some(self.root.clone()),
            current_track: self.current.map(|index| self.tracks[index].path.clone()),
            volume: self.volume,
            playback_mode: self.playback_mode,
        }
    }

    fn scan_message(&mut self) {
        self.message = if self.warnings.is_empty() {
            format!(
                "已扫描 {} 首歌曲；Enter 播放，? 查看帮助",
                self.tracks.len()
            )
        } else {
            format!(
                "已扫描 {} 首；跳过 {} 项：{}",
                self.tracks.len(),
                self.warnings.len(),
                self.warnings[0]
            )
        };
    }

    /// 按相对路径进行不区分大小写的筛选，并选中首个匹配项。
    pub fn filter(&mut self) {
        let query = self.query.to_lowercase();
        self.visible = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                track
                    .path
                    .strip_prefix(&self.root)
                    .unwrap_or(&track.path)
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.table.select(if self.visible.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// 将当前界面行号转换为音乐库索引；没有选中项时返回空。
    pub fn selected(&self) -> Option<usize> {
        self.table
            .selected()
            .and_then(|index| self.visible.get(index).copied())
    }

    fn move_selection(&mut self, delta: isize) {
        if !self.visible.is_empty() {
            let selected = self.table.selected().unwrap_or(0) as isize;
            let index = (selected + delta).clamp(0, self.visible.len() as isize - 1);
            self.table.select(Some(index as usize));
        }
    }

    // 只有音频加载成功后才更新当前歌曲，避免失败时界面与声音不一致。
    fn play_index(&mut self, index: usize) -> Result<()> {
        let track = &self.tracks[index];
        if self.player.is_none() {
            self.player = Some(Player::new()?);
        }
        self.player
            .as_mut()
            .unwrap()
            .load(&track.path, self.volume)?;
        self.current = Some(index);
        self.active = true;
        if let Some(row) = self.visible.iter().position(|item| *item == index) {
            self.table.select(Some(row));
        }
        self.message = format!("正在播放：{}", track.path.display());
        Ok(())
    }

    // 手动切歌始终使用完整音乐库顺序，并在首尾循环。
    fn adjacent(&self, backwards: bool) -> Option<usize> {
        if self.tracks.is_empty() {
            return None;
        }
        match self.current {
            Some(index) if backwards => Some((index + self.tracks.len() - 1) % self.tracks.len()),
            Some(index) => Some((index + 1) % self.tracks.len()),
            None => self.selected(),
        }
    }

    // 用文件路径重新定位当前歌曲；文件消失时释放播放器并结束播放。
    fn rescan(&mut self) -> Result<()> {
        let library = library::scan(&self.root)?;
        let current_path = self.current.map(|index| self.tracks[index].path.clone());
        self.tracks = library.tracks;
        self.warnings = library.warnings;
        self.current =
            current_path.and_then(|path| self.tracks.iter().position(|track| track.path == path));
        if self.current.is_none() {
            self.player = None;
            self.active = false;
        }
        self.filter();
        self.scan_message();
        Ok(())
    }

    // 自然续播使用完整音乐库；空库或失效的当前索引不再续播。
    fn next_after_finish(&self) -> Option<usize> {
        let index = self.current.filter(|index| *index < self.tracks.len())?;
        match self.playback_mode {
            PlaybackMode::Sequential => (index + 1 < self.tracks.len()).then_some(index + 1),
            PlaybackMode::RepeatAll => Some((index + 1) % self.tracks.len()),
            PlaybackMode::RepeatOne => Some(index),
        }
    }

    /// 检查播放结束并按模式续播；加载失败时停止，等待用户操作。
    pub fn tick(&mut self) {
        if self.active && self.player.as_ref().map(Player::finished).unwrap_or(false) {
            self.active = false;
            if let Some(player) = &mut self.player {
                player.stop();
            }
            if let Some(next) = self.next_after_finish() {
                if let Err(error) = self.play_index(next) {
                    self.message = format!("{:#}", error);
                }
            } else {
                self.message = "列表播放完毕；Enter 重新播放，n/p 切歌".into();
            }
        }
    }

    /// 按搜索、帮助和普通模式处理按键；返回真表示请求退出。
    pub fn handle_key(&mut self, key: Key) -> bool {
        if key == Key::Ctrl('c') {
            return true;
        }
        // 输入模式优先消费字符，避免搜索文字触发播放快捷键。
        if self.searching {
            match key {
                Key::Enter => self.searching = false,
                Key::Esc => {
                    self.searching = false;
                    self.query.clear();
                    self.filter();
                }
                Key::Backspace => {
                    self.query.pop();
                    self.filter();
                }
                Key::Char(c) => {
                    self.query.push(c);
                    self.filter();
                }
                _ => {}
            }
            return false;
        }
        let keys = self.user_config.keys.clone();
        if self.help {
            if key == keys.help || key == keys.back || key == Key::Esc {
                self.help = false;
            }
            return false;
        }
        if key == keys.back {
            return true;
        }
        let result: Result<()> = (|| {
            if key == keys.help {
                self.help = true;
            } else if key == keys.search {
                self.searching = true;
            } else if key == Key::Esc {
                self.query.clear();
                self.filter();
            } else if key == keys.submit {
                if let Some(index) = self.selected() {
                    self.play_index(index)?;
                }
            } else if key == keys.next_track || key == keys.previous_track {
                if let Some(index) = self.adjacent(key == keys.previous_track) {
                    self.play_index(index)?;
                }
            } else if key == keys.toggle_playback {
                if self.active {
                    if let Some(player) = &mut self.player {
                        player.toggle();
                    }
                } else if let Some(index) = self.current.or_else(|| self.selected()) {
                    self.play_index(index)?;
                }
            } else if key == keys.increase_volume || key == keys.decrease_volume {
                let step = self.user_config.behavior.volume_increment as f32 / 100.0;
                self.volume = (self.volume
                    + if key == keys.increase_volume {
                        step
                    } else {
                        -step
                    })
                .clamp(0.0, 1.0);
                if let Some(player) = &self.player {
                    player.set_volume(self.volume);
                }
            } else if key == keys.reset_play {
                self.rescan()?;
            } else if key == keys.cycle_playback_mode {
                self.playback_mode = self.playback_mode.next();
            } else if key == Key::Down || key == Key::Char('j') {
                self.move_selection(1);
            } else if key == Key::Up || key == Key::Char('k') {
                self.move_selection(-1);
            } else if key == Key::PageDown || key == keys.next_page {
                self.move_selection(10);
            } else if key == Key::PageUp || key == keys.previous_page {
                self.move_selection(-10);
            } else if key == Key::Home || key == keys.jump_to_start {
                self.move_selection(-(self.visible.len() as isize));
            } else if key == Key::End || key == keys.jump_to_end {
                self.move_selection(self.visible.len() as isize);
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.message = format!("{:#}", error);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playback_modes_choose_next_track_and_preserve_manual_navigation() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        for mode in [
            PlaybackMode::Sequential,
            PlaybackMode::RepeatAll,
            PlaybackMode::RepeatOne,
        ] {
            app.playback_mode = mode;
            assert_eq!(app.next_after_finish(), None);
        }
        for name in ["a.wav", "b.wav", "c.wav"] {
            std::fs::write(temp.path().join(name), b"").unwrap();
        }
        app.rescan().unwrap();
        app.playback_mode = PlaybackMode::Sequential;
        app.current = Some(1);
        assert_eq!(app.next_after_finish(), Some(2));
        app.current = Some(2);
        assert_eq!(app.next_after_finish(), None);
        for (mode, next) in [(PlaybackMode::RepeatAll, 0), (PlaybackMode::RepeatOne, 2)] {
            app.handle_key(Key::Char('m'));
            assert_eq!(app.playback_mode, mode);
            assert_eq!(app.next_after_finish(), Some(next));
            assert_eq!(app.adjacent(false), Some(0));
            assert_eq!(app.adjacent(true), Some(1));
        }
        app.handle_key(Key::Char('m'));
        assert_eq!(app.playback_mode, PlaybackMode::Sequential);
        app.handle_key(Key::Char('/'));
        app.handle_key(Key::Char('m'));
        assert_eq!(app.query, "m");
        assert_eq!(app.playback_mode, PlaybackMode::Sequential);
        app.handle_key(Key::Esc);
        app.tracks.truncate(1);
        app.current = Some(0);
        for (mode, expected) in [
            (PlaybackMode::Sequential, None),
            (PlaybackMode::RepeatAll, Some(0)),
            (PlaybackMode::RepeatOne, Some(0)),
        ] {
            app.playback_mode = mode;
            assert_eq!(app.next_after_finish(), expected);
        }
    }

    #[test]
    fn navigation_filtering_and_rescan() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["a.wav", "b.wav", "歌曲.wav"] {
            std::fs::write(temp.path().join(name), b"").unwrap();
        }
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        assert_eq!(app.selected(), Some(0));
        app.handle_key(Key::Down);
        assert_eq!(app.selected(), Some(1));
        app.current = Some(2);
        assert_eq!(app.adjacent(false), Some(0));
        app.current = Some(0);
        assert_eq!(app.adjacent(true), Some(2));
        app.query = "歌曲".into();
        app.filter();
        assert_eq!(app.selected(), Some(2));
        app.query = "missing".into();
        app.filter();
        app.handle_key(Key::Down);
        assert_eq!(app.selected(), None);
        app.handle_key(Key::Esc);
        assert_eq!(app.visible.len(), 3);
        std::fs::remove_file(temp.path().join("a.wav")).unwrap();
        app.rescan().unwrap();
        assert_eq!(app.current, None);
        assert_eq!(app.tracks.len(), 2);
    }
    #[test]
    fn restores_by_path_without_playing_and_handles_missing_track() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["a.wav", "b.wav"] {
            std::fs::write(temp.path().join(name), b"").unwrap();
        }
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        let saved = SessionState {
            music_directory: Some(app.root.clone()),
            current_track: Some(app.tracks[1].path.clone()),
            volume: 0.4,
            playback_mode: PlaybackMode::RepeatOne,
        };
        app.restore(&saved);
        assert_eq!(app.selected(), Some(1));
        assert_eq!(app.current, Some(1));
        assert!(!app.active);
        assert!(app.player.is_none());
        app.handle_key(Key::Up);
        assert_eq!(app.session_state(), saved);
        std::fs::remove_file(temp.path().join("b.wav")).unwrap();
        let mut reopened = App::new(temp.path().into(), UserConfig::new()).unwrap();
        reopened.restore(&saved);
        assert_eq!(reopened.selected(), Some(0));
        assert_eq!(reopened.current, None);
        assert_eq!(reopened.volume, 0.4);
    }

    #[test]
    fn empty_library_controls_are_safe() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        for key in [
            Key::Enter,
            Key::Char(' '),
            Key::Char('n'),
            Key::Char('p'),
            Key::Down,
        ] {
            assert!(!app.handle_key(key));
        }
        assert!(app.player.is_none());
        assert!(app.handle_key(Key::Char('q')));
    }
    #[test]
    #[ignore = "requires an audio output device"]
    fn local_playback_controls_and_auto_advance() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["a.wav", "b.wav"] {
            crate::player::tests::wav(&temp.path().join(name));
        }
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        app.volume = 0.0;
        app.handle_key(Key::Enter);
        assert_eq!(app.current, Some(0), "{}", app.message);
        app.handle_key(Key::Char(' '));
        assert!(app.player.as_ref().unwrap().is_paused());
        app.handle_key(Key::Char('n'));
        assert_eq!(app.current, Some(1));
        app.handle_key(Key::Char('p'));
        assert_eq!(app.current, Some(0));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
        while app.active && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
            app.tick();
        }
        assert_eq!(app.current, Some(1));
        assert!(!app.active, "playlist must stop after the final track");
    }
}
