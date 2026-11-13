use std::process::Command;

#[test]
fn lists_recursive_audio_files_without_a_terminal_or_audio_device() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("专辑")).unwrap();
    let track = temp.path().join("专辑/歌曲.WAV");
    std::fs::write(&track, b"").unwrap();
    std::fs::write(temp.path().join("cover.jpg"), b"").unwrap();
    let config = temp.path().join("config.toml");
    std::fs::write(&config, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rs-music-tui"))
        .arg("--dir")
        .arg(temp.path())
        .arg("--list")
        .arg("--config")
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listed = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        listed.trim(),
        track.canonicalize().unwrap().display().to_string()
    );
}

#[test]
fn invalid_directory_exits_with_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config.toml");
    std::fs::write(&config, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rs-music-tui"))
        .arg("--dir")
        .arg(temp.path().join("missing"))
        .arg("--list")
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("无法打开音乐目录"));
}

#[test]
fn configuration_uses_working_directory_and_explicit_override() {
    let temp = tempfile::tempdir().unwrap();
    let music = temp.path().join("music");
    std::fs::create_dir(&music).unwrap();
    // 音乐目录中的配置不能替代工作目录配置。
    std::fs::write(music.join("config.toml"), "[broken").unwrap();
    let run = |config: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rs-music-tui"));
        command
            .current_dir(temp.path())
            .arg("--dir")
            .arg(&music)
            .arg("--list");
        if let Some(path) = config {
            command.arg("--config").arg(path);
        }
        command.output().unwrap()
    };
    assert!(run(None).status.success());
    let default_path = temp.path().join("config.toml");
    std::fs::write(&default_path, "[behavior]\nvolume_increment = 101").unwrap();
    let output = run(None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("音量步长"));
    // 显式配置优先，且相对路径也以工作目录为基准。
    std::fs::write(
        temp.path().join("custom.toml"),
        "[theme]\nselected = 'LightBlue'",
    )
    .unwrap();
    assert!(run(Some("custom.toml")).status.success());
    let output = run(Some("missing.toml"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("无法读取配置"));
    std::fs::write(&default_path, "").unwrap();
    assert!(run(None).status.success());
}

#[test]
fn saved_directory_precedence_fallback_and_read_only_list() {
    let cwd = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    let override_dir = tempfile::tempdir().unwrap();
    for (directory, name) in [
        (cwd.path(), "fallback.wav"),
        (music.path(), "saved.wav"),
        (override_dir.path(), "explicit.wav"),
    ] {
        std::fs::write(directory.join(name), b"").unwrap();
    }
    let state_path = cwd.path().join("state.toml");
    let run = |directory: Option<&std::path::Path>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rs-music-tui"));
        command.current_dir(cwd.path()).arg("--list");
        if let Some(directory) = directory {
            command.arg("--dir").arg(directory);
        }
        command.output().unwrap()
    };
    assert!(run(None).status.success());
    assert!(!state_path.exists());
    let state = format!(
        "music_directory = '{}'\nvolume = 0.6\n",
        music.path().display()
    );
    std::fs::write(&state_path, &state).unwrap();
    let output = run(None);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("saved.wav"));
    let output = run(Some(override_dir.path()));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("explicit.wav"));
    assert!(!run(Some(&cwd.path().join("missing"))).status.success());
    assert_eq!(std::fs::read_to_string(&state_path).unwrap(), state);
    music.close().unwrap();
    let output = run(None);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fallback.wav"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("保存的音乐目录不可用"));
    assert_eq!(std::fs::read_to_string(&state_path).unwrap(), state);
    std::fs::write(&state_path, "[broken").unwrap();
    let output = run(None);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("无效的播放状态文件"));
    assert_eq!(std::fs::read_to_string(&state_path).unwrap(), "[broken");
}
