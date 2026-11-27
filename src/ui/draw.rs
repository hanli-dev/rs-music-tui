use crate::app::{App, MessageLevel};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap};
use std::time::Duration;

/// 绘制搜索、音乐库、歌曲表格和播放条，小窗口显示尺寸提示。
pub fn draw_main_layout(frame: &mut Frame<'_>, app: &mut App) {
    let theme = app.user_config.theme;
    if frame.area().width < 40 || frame.area().height < 12 {
        frame.render_widget(
            Paragraph::new("窗口太小，请放大至至少 40×12；q 退出")
                .style(Style::default().fg(theme.hint)),
            frame.area(),
        );
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(6),
            ]
            .as_ref(),
        )
        .split(frame.area());
    let search = if app.query.is_empty() && !app.searching {
        "按 / 筛选本地歌曲".to_string()
    } else {
        format!("{}{}", app.query, if app.searching { "▏" } else { "" })
    };
    frame.render_widget(
        Paragraph::new(search)
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("搜索 /  |  ? 帮助")
                    .border_style(Style::default().fg(if app.searching {
                        theme.active
                    } else {
                        theme.inactive
                    })),
            ),
        chunks[0],
    );

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(78)].as_ref())
        .split(chunks[1]);
    let side = vec![
        Line::from(Span::styled(
            "♫ 本地音乐",
            Style::default()
                .fg(theme.banner)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("全部歌曲  {}", app.tracks.len())),
        Line::from(format!("当前显示  {}", app.visible.len())),
        Line::from(""),
        Line::styled(
            app.root.display().to_string(),
            Style::default().fg(theme.hint),
        ),
        Line::from(""),
        Line::styled(
            format!("{} 重新扫描", app.user_config.keys.reset_play),
            Style::default().fg(theme.hint),
        ),
        Line::styled(
            "MP3 / FLAC / WAV / OGG / M4A",
            Style::default().fg(theme.hint),
        ),
    ];
    frame.render_widget(
        Paragraph::new(side)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("音乐库")
                    .border_style(Style::default().fg(theme.inactive)),
            ),
        body[0],
    );

    let rows = app
        .visible
        .iter()
        .map(|index| {
            let track = &app.tracks[*index];
            // 播放中的整行统一强调；选中背景和前景由表格最后覆盖，保留粗体。
            let playing = app.active && app.current == Some(*index);
            let row_style = if playing {
                Style::default()
                    .fg(theme.active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let detail_style = if playing {
                row_style
            } else {
                Style::default().fg(theme.hint)
            };
            let marker = if app.current == Some(*index) {
                "▶"
            } else {
                ""
            };
            let relative = track.path.strip_prefix(&app.root).unwrap_or(&track.path);
            let folder = relative
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let format = track
                .path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_uppercase();
            Row::new(vec![
                Cell::from(marker).style(Style::default().fg(theme.active)),
                Cell::from(track.title.clone()),
                Cell::from(folder).style(detail_style),
                Cell::from(format).style(detail_style),
            ])
            .style(row_style)
        })
        .collect::<Vec<_>>();
    // 先预留边框、选择标记、间距和固定列宽，防止歌曲格式列被挤出。
    let flexible_width = body[1].width.saturating_sub(14);
    let title_width = flexible_width * 3 / 5;
    let widths = [
        Constraint::Length(2),
        Constraint::Length(title_width),
        Constraint::Length(flexible_width - title_width),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths)
        .header(Row::new(vec!["", "歌曲", "目录", "格式"]).style(Style::default().fg(theme.header)))
        .column_spacing(1)
        .highlight_symbol("> ")
        .row_highlight_style(
            Style::default()
                .fg(theme.selected)
                .bg(theme.selected_background),
        )
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("歌曲列表")
                .border_style(Style::default().fg(if app.searching || app.help {
                    theme.inactive
                } else {
                    theme.active
                })),
        );
    frame.render_stateful_widget(table, body[1], &mut app.table);
    if app.visible.is_empty() {
        let message = if app.tracks.is_empty() {
            "目录中没有支持的音频文件"
        } else {
            "没有匹配的歌曲；Esc 清除筛选"
        };
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(theme.hint)),
            Rect::new(
                body[1].x + 2,
                body[1].y + 2,
                body[1].width.saturating_sub(4),
                1,
            ),
        );
    }
    draw_playbar(frame, app, chunks[2]);
    if app.help {
        draw_help(frame, app);
    }
}

fn draw_playbar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.user_config.theme;
    let state = if !app.active {
        "Stopped"
    } else if app
        .player
        .as_ref()
        .map(|player| player.is_paused())
        .unwrap_or(false)
    {
        "Paused"
    } else {
        "Playing"
    };
    let title = app
        .current
        .map(|index| app.tracks[index].title.as_str())
        .unwrap_or("选择歌曲后按 Enter 播放");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(
                "{}  音量 {}%  |  {}",
                state,
                (app.volume * 100.0).round(),
                app.playback_mode.label()
            ),
            Style::default().fg(theme.playbar_text),
        ))
        .border_style(Style::default().fg(theme.inactive));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(inner);
    frame.render_widget(
        Paragraph::new(title).style(
            Style::default()
                .fg(theme.playbar_text)
                .add_modifier(Modifier::BOLD),
        ),
        rows[0],
    );
    let position = app
        .player
        .as_ref()
        .map(|player| player.position())
        .unwrap_or_default();
    let duration = app.player.as_ref().and_then(|player| player.duration);
    let ratio = duration
        .filter(|value| !value.is_zero())
        .map(|value| (position.as_secs_f64() / value.as_secs_f64()).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let label = format!(
        "{} / {}",
        timestamp(position),
        duration.map(timestamp).unwrap_or_else(|| "--:--".into())
    );
    frame.render_widget(
        Gauge::default()
            .ratio(ratio)
            .gauge_style(
                Style::default()
                    .fg(theme.playbar_progress)
                    .bg(theme.playbar_background),
            )
            .label(Span::styled(
                label,
                // 只设置文字颜色，让标签沿用所在位置的已播放或未播放背景。
                Style::default().fg(theme.playbar_progress_text),
            )),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(app.message.as_str()).style(Style::default().fg(match app.message_level {
            MessageLevel::Info => theme.hint,
            MessageLevel::Warning => theme.warning,
            MessageLevel::Error => theme.error,
        })),
        rows[2],
    );
    let keys = &app.user_config.keys;
    frame.render_widget(
        Paragraph::new(format!(
            "↑↓ 选择  {} 播放/暂停  {} 下一首  {} 上一首  {} 退出",
            keys.toggle_playback, keys.next_track, keys.previous_track, keys.back
        ))
        .style(Style::default().fg(theme.hint)),
        rows[3],
    );
}

fn timestamp(duration: Duration) -> String {
    format!(
        "{:02}:{:02}",
        duration.as_secs() / 60,
        duration.as_secs() % 60
    )
}

fn draw_help(frame: &mut Frame<'_>, app: &App) {
    let size = frame.area();
    let width = size.width.saturating_sub(4).min(74);
    let height = size.height.saturating_sub(2).min(15);
    let area = Rect::new(
        (size.width - width) / 2,
        (size.height - height) / 2,
        width,
        height,
    );
    let keys = &app.user_config.keys;
    let lines = vec![
        "↑/↓ 或 j/k：选择歌曲；PageUp/PageDown：翻页".to_string(),
        format!("{}：播放选中歌曲", keys.submit),
        format!("{}：暂停 / 继续", keys.toggle_playback),
        format!(
            "{} / {}：下一首 / 上一首（按完整音乐库顺序）",
            keys.next_track, keys.previous_track
        ),
        format!(
            "{} / {}：降低 / 提高音量",
            keys.decrease_volume, keys.increase_volume
        ),
        format!("{}：筛选文件名或目录；Enter 确认；Esc 清除", keys.search),
        format!("{}：重新扫描当前目录", keys.reset_play),
        format!(
            "{}：顺序播放 / 列表循环 / 单曲循环",
            keys.cycle_playback_mode
        ),
        "手动切歌到达首尾时循环；损坏文件会显示错误。".into(),
        format!(
            "{} / Esc：关闭帮助；{} / Ctrl-C：退出",
            keys.help, keys.back
        ),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(app.user_config.theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("本地播放器 · 帮助")
                    .border_style(Style::default().fg(app.user_config.theme.active)),
            ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::user_config::UserConfig;
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn playbar_displays_each_playback_mode() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        for label in ["顺序播放", "列表循环", "单曲循环"] {
            terminal
                .draw(|frame| draw_main_layout(frame, &mut app))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text = (17..23)
                .flat_map(|y| (0..80).map(move |x| (x, y)))
                .map(|position| buffer[position].symbol())
                .collect::<String>();
            // 宽字符的占位单元格为空格，去除后检查中文模式名称。
            assert!(text.replace(' ', "").contains(label));
            app.handle_key(crate::event::Key::Char('m'));
        }
    }

    #[test]
    fn format_column_fits_a_standard_terminal() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("song.wav"), b"").unwrap();
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| draw_main_layout(frame, &mut app))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rows = (0..24)
            .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>();
        assert!(
            rows.iter()
                .any(|row| row.contains("song") && row.contains("WAV"))
        );
    }

    #[test]
    fn layout_handles_empty_library_help_and_small_terminals() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().into(), UserConfig::new()).unwrap();
        for (width, height) in [(120, 30), (80, 24), (40, 12), (10, 3)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| draw_main_layout(frame, &mut app))
                .unwrap();
            app.help = true;
            terminal
                .draw(|frame| draw_main_layout(frame, &mut app))
                .unwrap();
            app.help = false;
        }
    }
}
