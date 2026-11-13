use crate::app::App;
use crate::event::Key;
use crate::state::StateWriter;
use anyhow::{Context, Result};
use crossterm::cursor::Show;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
};
use crossterm::{event, execute};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};
mod draw;

// 正常退出、返回错误或栈展开时恢复终端模式与光标。
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
    }
}

/// 在交互式终端运行绘制与事件循环，退出时恢复终端状态。
pub fn start_ui(app: &mut App, state_path: PathBuf) -> Result<()> {
    use std::io::IsTerminal;
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "请在交互式终端运行；使用 --list 可仅列出本地音乐"
    );
    enable_raw_mode().context("无法进入终端交互模式")?;
    let _guard = TerminalGuard;
    execute!(stdout(), EnterAlternateScreen)?;
    if app.user_config.behavior.set_window_title {
        execute!(stdout(), SetTitle("Local Music - TUI"))?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.hide_cursor()?;
    let mut writer = StateWriter::new(state_path);
    let result = (|| -> Result<()> {
        loop {
            app.tick();
            if let Err(error) = writer.sync(app.session_state(), Instant::now(), false) {
                app.message = format!("保存状态失败：{error:#}");
            }
            terminal.draw(|frame| draw::draw_main_layout(frame, app))?;
            if event::poll(Duration::from_millis(
                app.user_config.behavior.tick_rate_milliseconds,
            ))? {
                if let event::Event::Key(key) = event::read()? {
                    // 忽略按键松开事件，避免一次操作触发两次播放或切歌。
                    if key.kind != event::KeyEventKind::Release && app.handle_key(Key::from(key)) {
                        break;
                    }
                }
            }
        }
        Ok(())
    })();
    // 即使绘制或事件读取失败，也尝试保存最后状态，再恢复终端。
    let saved = writer.sync(app.session_state(), Instant::now(), true);
    drop(terminal);
    drop(_guard);
    if let Err(error) = saved {
        eprintln!("保存状态失败：{error:#}");
    }
    result
}
