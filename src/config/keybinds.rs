use crate::event::Key;

/// 各交互动作对应的快捷键；方向键和搜索输入由应用单独处理。
#[derive(Clone)]
pub struct KeyBindings {
    pub back: Key,
    pub next_page: Key,
    pub previous_page: Key,
    pub jump_to_start: Key,
    pub jump_to_end: Key,
    pub decrease_volume: Key,
    pub increase_volume: Key,
    pub toggle_playback: Key,
    pub next_track: Key,
    pub previous_track: Key,
    pub help: Key,
    pub search: Key,
    pub submit: Key,
    /// 重新扫描当前音乐目录。
    pub reset_play: Key,
    /// 循环切换自然续播模式。
    pub cycle_playback_mode: Key,
}
impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            back: Key::Char('q'),
            next_page: Key::Ctrl('d'),
            previous_page: Key::Ctrl('u'),
            jump_to_start: Key::Ctrl('a'),
            jump_to_end: Key::Ctrl('e'),
            decrease_volume: Key::Char('-'),
            increase_volume: Key::Char('='),
            toggle_playback: Key::Char(' '),
            next_track: Key::Char('n'),
            previous_track: Key::Char('p'),
            help: Key::Char('?'),
            search: Key::Char('/'),
            submit: Key::Enter,
            reset_play: Key::Char('r'),
            cycle_playback_mode: Key::Char('m'),
        }
    }
}
