use std::io;

use anyhow::{Result, anyhow};
use clap::{Arg, ArgAction, Command};
use clap_complete::{Shell, generate};

/// 定义命令行参数并生成各类终端的补全脚本。
pub struct ClapApplication {
    pub app: Command,
}

/// 命令行帮助中展示的应用标识。
pub const BANNER: &str =
    "██████╗ ███████╗      ███╗   ███╗██╗   ██╗███████╗██╗ ██████╗      ████████╗██╗   ██╗██╗
██╔══██╗██╔════╝      ████╗ ████║██║   ██║██╔════╝██║██╔════╝      ╚══██╔══╝██║   ██║██║
██████╔╝███████╗█████╗██╔████╔██║██║   ██║███████╗██║██║     █████╗   ██║   ██║   ██║██║
██╔══██╗╚════██║╚════╝██║╚██╔╝██║██║   ██║╚════██║██║██║     ╚════╝   ██║   ██║   ██║██║
██║  ██║███████║      ██║ ╚═╝ ██║╚██████╔╝███████║██║╚██████╗         ██║   ╚██████╔╝██║
╚═╝  ╚═╝╚══════╝      ╚═╝     ╚═╝ ╚═════╝ ╚══════╝╚═╝ ╚═════╝         ╚═╝    ╚═════╝ ╚═╝
                                                                                        ";

impl ClapApplication {
    /// 构建目录扫描、配置文件和补全参数定义。
    pub fn new() -> Self {
        ClapApplication {
            app: Command::new(env!("CARGO_PKG_NAME"))
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about(env!("CARGO_PKG_DESCRIPTION"))
                .before_help(BANNER)
                .after_help(
                    r"Example: rs-music-tui --dir D:\Music | Press ? in the player for help",
                )
                .arg(
                    Arg::new("completions")
                        .long("completions")
                        .help("Generates completions for your preferred shell")
                        .num_args(1)
                        .value_parser(["bash", "zsh", "fish", "power-shell", "elvish"])
                        .value_name("SHELL"),
                )
                .arg(
                    Arg::new("directory")
                        .short('d')
                        .long("dir")
                        .value_parser(clap::value_parser!(std::path::PathBuf))
                        .num_args(1)
                        .value_name("DIRECTORY")
                        .help("Recursively scan this music directory (default: saved directory or current directory)"),
                )
                .arg(
                    Arg::new("list")
                        .long("list")
                        .action(ArgAction::SetTrue)
                        .help("List local audio files without opening the player"),
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .value_parser(clap::value_parser!(std::path::PathBuf))
                        .num_args(1)
                        .help("Path to a TOML configuration (default: ./config.toml)"),
                ),
        }
    }

    /// 将指定 Shell 的补全脚本写入标准输出；不支持的名称返回错误。
    pub fn gen_completions(&mut self, completions: &str) -> Result<()> {
        let shell = match completions {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "power-shell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => return Err(anyhow!("no completions available for '{}'", completions)),
        };
        generate(shell, &mut self.app, "rs-music-tui", &mut io::stdout());
        Ok(())
    }
}
