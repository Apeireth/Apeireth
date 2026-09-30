//! 斜杠命令解析与 `/` 补全 (面板 4 会话管理的命令面)。

/// 解析后的命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// 打开帮助浮层。
    Help,
    /// 新建会话页签。
    New,
    /// 续接账本中的既有会话。
    Resume {
        /// 目标会话 id; 缺省只刷新账本。
        target: Option<String>,
    },
    /// 压缩当前会话上下文。
    Compact,
    /// 导出会话记录。
    Export {
        /// 导出路径; 缺省用默认文件名。
        path: Option<String>,
    },
    /// 查看 / 热切换模型。
    Model {
        /// 目标模型; 缺省只列出可用模型。
        name: Option<String>,
    },
    /// 时间倒带: 回到上一回合重新输入。
    Rewind,
    /// 切换动效档。
    Motion {
        /// `full` / `reduced`; 缺省只显示当前档。
        mode: Option<String>,
    },
    /// 退出驾驶舱。
    Quit,
}

/// 命令解析错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// 不是以 `/` 开头的命令。
    NotACommand,
    /// 只有 `/` 没有命令名。
    Empty,
    /// 未知命令。
    Unknown {
        /// 输入的命令名。
        name: String,
    },
    /// 参数不合法。
    BadArgument {
        /// 命令名。
        command: &'static str,
        /// 细节。
        detail: String,
    },
}

impl CommandError {
    /// 面向用户的错误文案。
    pub fn message(&self) -> String {
        match self {
            Self::NotACommand => "不是命令: 命令以 / 开头".to_string(),
            Self::Empty => "空命令: 输入 /help 查看命令表".to_string(),
            Self::Unknown { name } => format!("未知命令 /{name}: 输入 /help 查看命令表"),
            Self::BadArgument { command, detail } => {
                format!("命令 /{command} 参数不合法: {detail}")
            }
        }
    }
}

/// 命令规格 (补全与帮助共用一份)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// 命令名 (不含斜杠)。
    pub name: &'static str,
    /// 用法。
    pub usage: &'static str,
    /// 一句话说明。
    pub summary: &'static str,
}

/// 命令全集 (顺序即补全顺序)。
pub const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        usage: "/help",
        summary: "打开帮助浮层",
    },
    CommandSpec {
        name: "new",
        usage: "/new",
        summary: "新建会话页签",
    },
    CommandSpec {
        name: "resume",
        usage: "/resume [会话id]",
        summary: "续接账本中的既有会话",
    },
    CommandSpec {
        name: "compact",
        usage: "/compact",
        summary: "压缩当前会话上下文",
    },
    CommandSpec {
        name: "export",
        usage: "/export [路径]",
        summary: "导出会话记录 (Markdown)",
    },
    CommandSpec {
        name: "model",
        usage: "/model [模型]",
        summary: "查看 / 热切换模型",
    },
    CommandSpec {
        name: "rewind",
        usage: "/rewind",
        summary: "时间倒带: 回到上一回合重新输入",
    },
    CommandSpec {
        name: "motion",
        usage: "/motion [full|reduced]",
        summary: "切换动效档位",
    },
    CommandSpec {
        name: "quit",
        usage: "/quit",
        summary: "退出驾驶舱",
    },
];

/// 解析一行命令输入 (`/name [arg]`)。
pub fn parse_command(input: &str) -> Result<Command, CommandError> {
    let trimmed = input.trim();
    let Some(rest) = trimmed.strip_prefix('/') else {
        return Err(CommandError::NotACommand);
    };
    let rest = rest.trim_start();
    let (name, arg) = match rest.split_once(char::is_whitespace) {
        Some((name, arg)) => (name, Some(arg.trim())),
        None => (rest, None),
    };
    if name.is_empty() {
        return Err(CommandError::Empty);
    }
    let arg = arg.filter(|value| !value.is_empty());
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "help" => Ok(Command::Help),
        "new" => Ok(Command::New),
        "resume" => Ok(Command::Resume {
            target: arg.map(str::to_string),
        }),
        "compact" => Ok(Command::Compact),
        "export" => Ok(Command::Export {
            path: arg.map(str::to_string),
        }),
        "model" => Ok(Command::Model {
            name: arg.map(str::to_string),
        }),
        "rewind" => Ok(Command::Rewind),
        "motion" => match arg {
            None => Ok(Command::Motion { mode: None }),
            Some(value) => match crate::theme::MotionMode::parse(value) {
                Some(_) => Ok(Command::Motion {
                    mode: Some(value.to_ascii_lowercase()),
                }),
                None => Err(CommandError::BadArgument {
                    command: "motion",
                    detail: "只接受 full 或 reduced".to_string(),
                }),
            },
        },
        "quit" => Ok(Command::Quit),
        other => Err(CommandError::Unknown {
            name: other.to_string(),
        }),
    }
}

/// `/` 补全: 按前缀过滤命令规格 (前缀可带或不带斜杠)。
pub fn complete_command(prefix: &str) -> Vec<&'static CommandSpec> {
    let needle = prefix.trim().trim_start_matches('/').to_ascii_lowercase();
    COMMAND_SPECS
        .iter()
        .filter(|spec| spec.name.starts_with(&needle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 命令解析: 全部命令都解析成对应形状。
    #[test]
    fn parses_every_supported_command() {
        assert_eq!(parse_command("/help").unwrap(), Command::Help);
        assert_eq!(parse_command("/new").unwrap(), Command::New);
        assert_eq!(
            parse_command("/resume abc-123").unwrap(),
            Command::Resume {
                target: Some("abc-123".to_string())
            }
        );
        assert_eq!(
            parse_command("/resume").unwrap(),
            Command::Resume { target: None }
        );
        assert_eq!(parse_command("/compact").unwrap(), Command::Compact);
        assert_eq!(
            parse_command("  /export  out.md  ").unwrap(),
            Command::Export {
                path: Some("out.md".to_string())
            }
        );
        assert_eq!(
            parse_command("/model demo-model").unwrap(),
            Command::Model {
                name: Some("demo-model".to_string())
            }
        );
        assert_eq!(parse_command("/rewind").unwrap(), Command::Rewind);
        assert_eq!(
            parse_command("/motion reduced").unwrap(),
            Command::Motion {
                mode: Some("reduced".to_string())
            }
        );
        assert_eq!(parse_command("/quit").unwrap(), Command::Quit);
    }

    /// 命令解析: 非法输入给诚实错误。
    #[test]
    fn rejects_malformed_command_input() {
        assert_eq!(parse_command("hello"), Err(CommandError::NotACommand));
        assert_eq!(parse_command("   /  "), Err(CommandError::Empty));
        assert_eq!(
            parse_command("/nope"),
            Err(CommandError::Unknown {
                name: "nope".to_string()
            })
        );
        assert_eq!(
            parse_command("/motion calm"),
            Err(CommandError::BadArgument {
                command: "motion",
                detail: "只接受 full 或 reduced".to_string()
            })
        );
    }

    /// `/` 补全: 前缀过滤命中子集且保持命令表顺序。
    #[test]
    fn completes_commands_by_prefix() {
        let re: Vec<_> = complete_command("/re").iter().map(|s| s.name).collect();
        assert_eq!(re, vec!["resume", "rewind"]);

        let mo: Vec<_> = complete_command("mo").iter().map(|s| s.name).collect();
        assert_eq!(mo, vec!["model", "motion"]);

        assert_eq!(complete_command("/").len(), COMMAND_SPECS.len());
        assert!(complete_command("/zzz").is_empty());
    }
}
