use crate::app::App;
use crate::cli::clap::ClapApplication;
use crate::config::user_config::UserConfig;
use anyhow::Result;
use std::path::PathBuf;
mod app;
mod cli;
mod config;
mod event;
mod library;
mod player;
mod state;
mod ui;

fn main() -> Result<()> {
    // 补全命令独立执行，不读取用户配置或初始化音乐库。
    let mut cli = ClapApplication::new();
    let matches = cli.app.clone().get_matches();
    if let Some(shell) = matches.get_one::<String>("completions") {
        return cli.gen_completions(shell);
    }
    let mut config = UserConfig::new();
    config.path_to_config = matches.get_one::<PathBuf>("config").cloned();
    config.load_config()?;
    let cwd = std::env::current_dir()?;
    let state_path = cwd.join("state.toml");
    let mut notices = Vec::new();
    let saved = state::SessionState::load(&state_path).unwrap_or_else(|error| {
        notices.push(format!("{error:#}；使用默认播放状态"));
        state::SessionState::default()
    });
    // 显式目录错误直接返回；保存目录失效时才回退到工作目录。
    let mut app = if let Some(directory) = matches.get_one::<PathBuf>("directory") {
        App::new(directory.clone(), config)?
    } else if let Some(directory) = &saved.music_directory {
        match App::new(directory.clone(), config.clone()) {
            Ok(app) => app,
            Err(error) => {
                notices.push(format!(
                    "保存的音乐目录不可用：{error:#}；已使用当前工作目录"
                ));
                App::new(cwd, config)?
            }
        }
    } else {
        App::new(cwd, config)?
    };
    app.restore(&saved);
    for notice in &notices {
        app.message_level = app::MessageLevel::Warning;
        app.message.push_str("；");
        app.message.push_str(notice);
    }
    // 列表模式直接输出路径，不进入终端界面，也不打开音频设备。
    if matches.get_flag("list") {
        for track in &app.tracks {
            println!("{}", track.path.display());
        }
        for warning in app.warnings.iter().chain(&notices) {
            eprintln!("Warning: {}", warning);
        }
        return Ok(());
    }
    ui::start_ui(&mut app, state_path)
}
