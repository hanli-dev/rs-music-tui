use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// 本地音频文件及用于列表显示的文件名标题。
#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
}

/// 一次目录扫描的结果，包括可播放文件列表和可恢复的读取错误。
pub struct Library {
    pub root: PathBuf,
    pub tracks: Vec<Track>,
    pub warnings: Vec<String>,
}

/// 递归扫描支持的扩展名并按路径排序；根目录读取失败时报错，子项错误记录为警告。
/// 扫描阶段不解码音频，文件内容由播放器在加载时校验。
pub fn scan(directory: &Path) -> Result<Library> {
    let root = directory
        .canonicalize()
        .with_context(|| format!("无法打开音乐目录：{}", directory.display()))?;
    anyhow::ensure!(root.is_dir(), "不是目录：{}", root.display());
    let entries =
        fs::read_dir(&root).with_context(|| format!("无法读取音乐目录：{}", root.display()))?;
    let mut library = Library {
        root,
        tracks: Vec::new(),
        warnings: Vec::new(),
    };
    // 使用显式栈遍历目录，避免深层目录导致递归调用栈增长。
    let mut pending = vec![entries];
    while let Some(entries) = pending.pop() {
        for entry in entries {
            let result = (|| -> std::io::Result<()> {
                let entry = entry?;
                let kind = entry.file_type()?;
                let path = entry.path();
                // 跳过符号链接，避免循环遍历或扫描到指定目录之外。
                if kind.is_symlink() {
                    return Ok(());
                }
                if kind.is_dir() {
                    match fs::read_dir(&path) {
                        Ok(children) => pending.push(children),
                        Err(error) => {
                            library
                                .warnings
                                .push(format!("{}: {}", path.display(), error))
                        }
                    }
                } else if kind.is_file() && supported(&path) {
                    let title = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    library.tracks.push(Track { path, title });
                }
                Ok(())
            })();
            if let Err(error) = result {
                library.warnings.push(error.to_string());
            }
        }
    }
    library.tracks.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(library)
}

fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "wav" | "ogg" | "m4a"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recursive_scan_filters_and_preserves_paths() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("专辑")).unwrap();
        for name in [
            "b.MP3",
            "专辑/歌曲.wav",
            "a.flac",
            "c.ogg",
            "notes.txt",
            "cover.jpg",
        ] {
            fs::write(temp.path().join(name), b"").unwrap();
        }
        let library = scan(temp.path()).unwrap();
        assert_eq!(library.tracks.len(), 4);
        assert!(
            library
                .tracks
                .windows(2)
                .all(|pair| pair[0].path < pair[1].path)
        );
        assert!(library.tracks.iter().any(|track| track.title == "歌曲"));
        assert!(library.tracks.iter().all(|track| track.path.is_absolute()));
    }
    #[test]
    fn empty_and_invalid_directories() {
        let temp = tempfile::tempdir().unwrap();
        assert!(scan(temp.path()).unwrap().tracks.is_empty());
        assert!(scan(&temp.path().join("missing")).is_err());
        let file = temp.path().join("song.mp3");
        fs::write(&file, b"").unwrap();
        assert!(scan(&file).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn skips_symlink_loops() {
        let temp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(temp.path(), temp.path().join("loop")).unwrap();
        assert!(scan(temp.path()).unwrap().tracks.is_empty());
    }
}
