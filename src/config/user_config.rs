use crate::config::behavior::BehaviorConfig;
use crate::config::keybinds::KeyBindings;
use crate::config::theme::Theme;
use crate::event::Key;
use anyhow::{Context, Result, anyhow};
use ratatui::style::Color;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 汇总配置路径、交互行为、主题和快捷键。
#[derive(Clone)]
pub struct UserConfig {
    pub path_to_config: Option<PathBuf>,
    pub behavior: BehaviorConfig,
    pub theme: Theme,
    pub keys: KeyBindings,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigFile {
    keybindings: HashMap<String, String>,
    behavior: BehaviorConfig,
    theme: HashMap<String, String>,
}

impl UserConfig {
    /// 创建默认配置，不读取或写入任何文件。
    pub fn new() -> Self {
        Self {
            path_to_config: None,
            behavior: BehaviorConfig::default(),
            theme: Theme::default(),
            keys: KeyBindings::default(),
        }
    }

    /// 读取 TOML 配置并校验行为参数、颜色及按键；缺省路径不存在时保留默认值。
    pub fn load_config(&mut self) -> Result<()> {
        let explicit = self.path_to_config.is_some();
        let path = match &self.path_to_config {
            Some(path) => path.clone(),
            None => std::env::current_dir()
                .context("无法获取当前工作目录")?
                .join("config.toml"),
        };
        if !explicit && !path.exists() {
            return Ok(());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("无法读取配置：{}", path.display()))?;
        if text.trim().is_empty() {
            return Ok(());
        }
        let config: ConfigFile = toml::from_str(&text)
            .with_context(|| format!("无效的 TOML 配置：{}", path.display()))?;
        anyhow::ensure!(
            (1..1000).contains(&config.behavior.tick_rate_milliseconds),
            "刷新间隔必须为 1–999 毫秒"
        );
        anyhow::ensure!(
            config.behavior.volume_increment <= 100,
            "音量步长必须为 0–100"
        );
        self.behavior = config.behavior;
        // 仅覆盖已支持且显式提供的配置项，其余字段保持默认值。
        macro_rules! key {
            ($name:ident) => {
                if let Some(value) = config.keybindings.get(stringify!($name)) {
                    self.keys.$name = parse_key(value)?;
                }
            };
        }
        key!(back);
        key!(next_page);
        key!(previous_page);
        key!(jump_to_start);
        key!(jump_to_end);
        key!(decrease_volume);
        key!(increase_volume);
        key!(toggle_playback);
        key!(next_track);
        key!(previous_track);
        key!(help);
        key!(search);
        key!(submit);
        key!(reset_play);
        macro_rules! color {
            ($name:ident) => {
                if let Some(value) = config.theme.get(stringify!($name)) {
                    self.theme.$name = parse_color(value)?;
                }
            };
        }
        color!(active);
        color!(banner);
        color!(hint);
        color!(inactive);
        color!(playbar_background);
        color!(playbar_progress);
        color!(playbar_progress_text);
        color!(playbar_text);
        color!(selected);
        color!(text);
        color!(header);
        Ok(())
    }
}

fn parse_key(value: &str) -> Result<Key> {
    if value.chars().count() == 1 {
        return Ok(Key::Char(value.chars().next().unwrap()));
    }
    let lower = value.to_lowercase();
    if let Some((modifier, character)) = lower.split_once('-') {
        let mut chars = character.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            return match modifier {
                "ctrl" => Ok(Key::Ctrl(c)),
                "alt" => Ok(Key::Alt(c)),
                _ => Err(anyhow!("无效按键：{}", value)),
            };
        }
    }
    match lower.as_str() {
        "enter" => Ok(Key::Enter),
        "space" => Ok(Key::Char(' ')),
        "esc" | "escape" => Ok(Key::Esc),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "backspace" => Ok(Key::Backspace),
        "tab" => Ok(Key::Tab),
        _ => Err(anyhow!("无效按键：{}", value)),
    }
}

fn parse_color(value: &str) -> Result<Color> {
    Ok(match value {
        "Reset" => Color::Reset,
        "Black" => Color::Black,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "Gray" => Color::Gray,
        "DarkGray" => Color::DarkGray,
        "LightRed" => Color::LightRed,
        "LightGreen" => Color::LightGreen,
        "LightYellow" => Color::LightYellow,
        "LightBlue" => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan" => Color::LightCyan,
        "White" => Color::White,
        _ => {
            let rgb = value
                .split(',')
                .map(|part| part.trim().parse::<u8>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            anyhow::ensure!(rgb.len() == 3, "无效颜色：{}", value);
            Color::Rgb(rgb[0], rgb[1], rgb[2])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_partial_toml_with_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
[behavior]
volume_increment = 5
[keybindings]
next_track = "x"
[theme]
selected = "Cyan"
"#,
        )
        .unwrap();
        let mut config = UserConfig::new();
        config.path_to_config = Some(path);
        config.load_config().unwrap();
        assert_eq!(config.keys.next_track, Key::Char('x'));
        assert_eq!(config.keys.previous_track, Key::Char('p'));
        assert_eq!(config.theme.selected, Color::Cyan);
        assert_eq!(config.theme.text, Theme::default().text);
        assert_eq!(config.behavior.volume_increment, 5);
        assert_eq!(config.behavior.tick_rate_milliseconds, 250);
        assert!(config.behavior.set_window_title);
        assert!(parse_key("ctrl").is_err());
        assert!(parse_key("ctrl-").is_err());
        assert_eq!(parse_key("ctrl-c").unwrap(), Key::Ctrl('c'));
    }

    #[test]
    fn rejects_invalid_toml_and_configuration_values() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        for text in [
            "[broken",
            "[behavior]\nvolume_increment = 101",
            "[behavior]\ntick_rate_milliseconds = 0",
            "[behavior]\ntick_rate_milliseconds = 1000",
            "[keybindings]\nnext_track = 'invalid-key'",
            "[theme]\nselected = 'invalid-color'",
        ] {
            fs::write(&path, text).unwrap();
            let mut config = UserConfig::new();
            config.path_to_config = Some(path.clone());
            assert!(config.load_config().is_err(), "{text}");
        }
    }
}
