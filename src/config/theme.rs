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
    /// 选中行的背景色，与选中文字颜色分别配置。
    pub selected_background: Color,
    pub warning: Color,
    pub error: Color,
    pub text: Color,
    pub header: Color,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            active: Color::Cyan,
            banner: Color::Cyan,
            hint: Color::Gray,
            inactive: Color::DarkGray,
            playbar_background: Color::DarkGray,
            playbar_progress: Color::Cyan,
            playbar_progress_text: Color::White,
            playbar_text: Color::White,
            selected: Color::White,
            selected_background: Color::Cyan,
            warning: Color::Yellow,
            error: Color::Red,
            text: Color::White,
            header: Color::Cyan,
        }
    }
}
