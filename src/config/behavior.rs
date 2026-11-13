use serde::Deserialize;

/// 控制音量调整步长、界面刷新间隔和终端标题。
#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    /// 每次调节的音量百分比，范围为 0 至 100。
    pub volume_increment: u8,
    /// 事件轮询等待上限，范围为 1 至 999 毫秒。
    pub tick_rate_milliseconds: u64,
    pub set_window_title: bool,
}
impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            volume_increment: 10,
            tick_rate_milliseconds: 250,
            set_window_title: true,
        }
    }
}
