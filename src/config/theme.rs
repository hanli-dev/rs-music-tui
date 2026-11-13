use ratatui::style::Color;

/// 终端各界面区域及交互状态使用的颜色。
#[derive(Copy, Clone, Debug)]
pub struct Theme {
    pub active: Color,
    pub banner: Color,
    pub hint: Color,
    pub inactive: Color,
    pub playbar_background: Color,
    pub playbar_progress: Color,
    pub playbar_progress_text: Color,
    pub playbar_text: Color,
    pub selected: Color,
    pub text: Color,
    pub header: Color,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            active: Color::LightBlue,
            banner: Color::LightCyan,
            hint: Color::Yellow,
            inactive: Color::LightYellow,
            playbar_background: Color::Black,
            playbar_progress: Color::Green,
            playbar_progress_text: Color::Yellow,
            playbar_text: Color::LightYellow,
            selected: Color::LightBlue,
            text: Color::White,
            header: Color::LightBlue,
        }
    }
}
