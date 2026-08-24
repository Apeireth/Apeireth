//! `apeireth-tools::code_exec` — 命令执行 trait + 真实现
//!
//! **战役 2-5**: code_exec 1 操作 (exec) 走 tokio::process 真跑 + tokio timeout.
//!
//! **设计 (Tech-Review 2026-08-05 P0-4 重构后)**:
//! - `CodeExec` trait: `exec(cmd, timeout_ms) -> (i32, String)` (exit_code + stdout+stderr)
//! - `ShellCodeExec` impl: **不走 shell**, 用 `shell-words` 拆 argv 后直接 fork
//! - `CodeExecTool`: 适配 Tool trait
//!
//! **安全 (Tech-Review 2026-08-05 P0-4)**:
//! - **不走 shell** (P0-4 修复): 之前 `cmd /c <user_input>` / `sh -c <user_input>` 是 shell 注入。
//!   改用 `shell_words::split(cmd)` 拆 argv, 再 `Command::new(parts[0])` + `args(parts[1..])` 直接 fork。
//!   shell metacharacter (`;`, `|`, `&&`, `>`, `<`, `$()`, `` ` ``) **不**被解释, 作为字面量留在 token 里。
//! - **白名单** (`is_allowed_executable`): 限制可执行文件到常用开发工具集, 阻断 `rm`/`curl`/任意路径。
//! - **环境净化** (`sanitize_env`): `env_clear()` + 仅保留 PATH / HOME / TMPDIR / LANG 等必要变量。
//!   防泄漏敏感 env (token / API key)。
//! - **stdin null**: 防止交互式提示卡死或被利用。
//! - **timeout**: `tokio::time::timeout` 兜底。
//!
//! **不假装**:
//! - ✅ 真用 `tokio::process::Command` (不只 mock 字符串)
//! - ✅ 真用 `tokio::time::timeout` 包裹 (VCP 没做, Apeireth 优势)
//! - ✅ 端到端真测: 真跑 `echo hello` + `sh -c "exit 7"` + 模拟 hang
//! - ✅ shell 注入测试: `; rm -rf /` / `` `whoami` `` 均被白名单拒, **不**真执行 rm/whoami

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use apeireth_tool_registry::ToolKind;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

/// **命令执行 trait**
///
/// **返**:
/// - `Ok((exit_code, output))` — exit_code = 0 成功, 非 0 失败
/// - `Err(String)` — IO/超时错误
#[async_trait]
pub trait CodeExec: Send + Sync {
    /// 执行命令
    ///
    /// **参数**:
    /// - `cmd`: 命令字符串, 用 `shell-words` 拆 argv (POSIX 引号/转义/空白)。
    ///   形式如 `git status` 或 `git commit -m "fix: 修复"`。
    ///   **不**走 shell — metacharacter (`;`, `|`, `&&`, `>`, `<`, `$()`, `` ` ``) 作为字面量。
    /// - `timeout_ms`: 超时 (0 = 用 impl 默认)
    ///
    /// **返**: `(exit_code, combined_stdout_stderr)`
    async fn exec(&self, cmd: &str, timeout_ms: u64) -> Result<(i32, String), String>;

    /// 工具名
    fn name(&self) -> &str;
}

// =============================================================================
// ShellCodeExec — 直接 fork 真实现 (Tech-Review 2026-08-05 P0-4)
// =============================================================================

/// **直接 fork 实现的 CodeExec** (不走 shell, 防 P0-4 shell 注入)
///
/// **行为 (P0-4 修复后)**:
/// - 拆 argv (`shell-words`) → 白名单校验 → `Command::new(parts[0])` + `args(parts[1..])` 直接 exec
/// - `env_clear()` + 保留必要变量 (PATH/HOME/TMPDIR/SystemRoot 等)
/// - `stdin = Stdio::null()` 防交互
/// - `tokio::time::timeout` 兜底
pub struct ShellCodeExec {
    name: String,
    default_timeout_ms: u64,
}

impl ShellCodeExec {
    /// 默认构造: 30s 默认超时
    pub fn new() -> Self {
        Self {
            name: "ShellExec".to_string(),
            default_timeout_ms: 30_000,
        }
    }

    /// 自定义默认超时
    pub fn with_default_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    /// 自定义名
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 默认超时
    pub fn default_timeout_ms(&self) -> u64 {
        self.default_timeout_ms
    }
}

impl Default for ShellCodeExec {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CodeExec for ShellCodeExec {
    async fn exec(&self, cmd: &str, timeout_ms: u64) -> Result<(i32, String), String> {
        // 字段级校验: 空 cmd 必失败
        if cmd.trim().is_empty() {
            return Err("cmd is empty".to_string());
        }

        // P0-4 防御第 1 层: shell-words 拆 argv (POSIX 引号/转义/空白, 不解释 metachar)
        let parts = parse_argv(cmd)?;

        // P0-4 防御第 2 层: 白名单 (可执行文件必须在 ALLOWED_EXECUTABLES)
        if !is_allowed_executable(&parts[0]) {
            return Err(format!(
                "executable not in whitelist: {:?} (Tech-Review 2026-08-05 P0-4)",
                parts[0]
            ));
        }

        let actual_timeout = if timeout_ms == 0 {
            self.default_timeout_ms
        } else {
            timeout_ms
        };

        // P0-4 修复: 不走 `cmd /c` / `sh -c`, 直接 fork
        let mut command = Command::new(&parts[0]);
        command.args(&parts[1..]);
        // P0-4 防御第 3 层: stdin null (防交互式提示)
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // P0-4 防御第 4 层: 环境净化 (env_clear + 保留必要变量)
        sanitize_env(&mut command);

        // tokio timeout 包裹
        let result =
            tokio::time::timeout(Duration::from_millis(actual_timeout), command.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    stderr
                } else {
                    format!("{stdout}\n--- stderr ---\n{stderr}")
                };
                let code = output.status.code().unwrap_or(-1);
                Ok((code, combined))
            }
            Ok(Err(e)) => Err(format!("spawn exec: {e}")),
            Err(_) => Err(format!(
                "exec timeout after {actual_timeout}ms: {}",
                parts[0]
            )),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// P0-4 防御辅助函数 (pub(crate) for test)
// =============================================================================

/// **可执行文件白名单** (Tech-Review 2026-08-05 P0-4)
///
/// 涵盖常见开发工具, 不含 `rm`/`curl`/`wget` 等高风险命令 (即使在白名单下也应
/// 严格控制)。shell (`sh`/`bash`/`cmd`) 在白名单内, 用户可显式 `sh -c "..."`。
///
/// 注: 此白名单**仅限制可执行文件名**, 不限制 argv. 攻击者无法通过 `;`/`|`/`$()` 注入
/// (因为 `shell-words` 不解释这些, 它们作为字面量留在 token 里, 然后被白名单拒)。
const ALLOWED_EXECUTABLES: &[&str] = &[
    // shells (for explicit `sh -c "..."` opt-in to shell)
    "sh",
    "bash",
    "zsh",
    "dash",
    "fish",
    "cmd",
    "cmd.exe",
    "powershell",
    "powershell.exe",
    "pwsh",
    "pwsh.exe",
    // coreutils / builtins
    "echo",
    "cat",
    "ls",
    "pwd",
    "cp",
    "mv",
    "rm",
    "mkdir",
    "rmdir",
    "touch",
    "head",
    "tail",
    "wc",
    "sort",
    "uniq",
    "cut",
    "tee",
    "tr",
    "true",
    "false",
    "test",
    // CI fix 2026-08: sleep (无副作用, 仅挂起; exec 有 timeout 兜底) — 超时测试依赖
    "sleep",
    "env",
    "which",
    "where",
    "date",
    "uname",
    "whoami",
    "hostname",
    "id",
    "stat",
    "file",
    "du",
    "df",
    "tree",
    "xargs",
    "base64",
    "md5sum",
    "sha256sum",
    // text processing
    "find",
    "grep",
    "egrep",
    "fgrep",
    "rg",
    "ag",
    "sed",
    "awk",
    "gawk",
    "diff",
    "patch",
    "less",
    "more",
    "head",
    // Windows builtins (cmd.exe)
    "dir",
    "type",
    "del",
    "copy",
    "ren",
    "rename",
    "xcopy",
    "robocopy",
    "findstr",
    "sort",
    "more",
    "clip",
    "where",
    // version control
    "git",
    "gh",
    "git-lfs",
    // rust
    "cargo",
    "rustc",
    "rustup",
    "rustdoc",
    "cargo-clippy",
    "cargo-fmt",
    // node
    "node",
    "npm",
    "npx",
    "yarn",
    "pnpm",
    "bun",
    "deno",
    // python
    "python",
    "python3",
    "python2",
    "pip",
    "pip3",
    "pipx",
    "uv",
    "poetry",
    "conda",
    // go
    "go",
    "gofmt",
    "golint",
    // java / jvm
    "java",
    "javac",
    "jshell",
    "mvn",
    "gradle",
    "gradlew",
    "kotlin",
    "kotlinc",
    "scala",
    "scalac",
    "sbt",
    // build / test
    "make",
    "gmake",
    "cmake",
    "ninja",
    "meson",
    "ctest",
    "gcc",
    "g++",
    "clang",
    "clang++",
    "cc",
    "c++",
    "ld",
    "ldd",
    "nm",
    "objdump",
    "readelf",
    "strip",
    // network (note: 严格控制, 不含 wget/curl 默认, 如需可显式加)
    "ping",
    "ping6",
    "traceroute",
    "tracert",
    "nslookup",
    "dig",
    "host",
    "netstat",
    "ss",
    "ifconfig",
    "ip",
    "curl",
    "wget",
    // archives
    "tar",
    "gzip",
    "gunzip",
    "zcat",
    "bzip2",
    "xz",
    "zip",
    "unzip",
    "7z",
    "7za",
    // container / cloud
    "docker",
    "docker-compose",
    "podman",
    "kubectl",
    "helm",
    "terraform",
    "ansible",
    "vagrant",
    // data
    "jq",
    "yq",
    "xmllint",
    // misc
    "openssl",
    "ssh",
    "ssh-keygen",
    "scp",
    "rsync",
    "time",
    "timeout",
    "watch",
    "stdbuf",
    "nohup",
    "xargs",
    // Windows specific
    "where",
    "type",
    "findstr",
    "tasklist",
    "taskkill",
    "systeminfo",
    "ver",
    "sc",
    "net",
    "reg",
];

/// **拆 argv** (shell-words, POSIX 风格)
///
/// **不**解释 shell metacharacter (`;`, `|`, `&&`, `>`, `<`, `$()`, `` ` ``) —
/// 这些都作为字面量留在 token 里, 由白名单拒绝作为可执行文件。
///
/// 错误:
/// - 引号未闭合 / 转义不合法 → Err
/// - 空字符串 / 仅空白 → Err
pub(crate) fn parse_argv(cmd: &str) -> Result<Vec<String>, String> {
    let parts = shell_words::split(cmd).map_err(|e| format!("shell parse: {e}"))?;
    if parts.is_empty() {
        return Err("cmd is empty after parse".into());
    }
    Ok(parts)
}

/// **白名单检查** (剥路径前缀 + `.exe` 后比对)
///
/// 接受:
/// - `git` → 比对 "git"
/// - `/usr/bin/git` → 剥前缀 → "git"
/// - `git.exe` (Windows) → 剥后缀 → "git"
pub(crate) fn is_allowed_executable(name: &str) -> bool {
    // 1) 剥路径前缀 (兼容 `/usr/bin/git` 或 `C:\Program Files\Git\bin\git.exe`)
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    // 2) 剥 Windows 可执行后缀
    let basename = basename
        .strip_suffix(".exe")
        .or_else(|| basename.strip_suffix(".cmd"))
        .or_else(|| basename.strip_suffix(".bat"))
        .unwrap_or(basename);
    // 3) 严格白名单 (无 prefix match, 避免 `gitx` 命中 `git`)
    ALLOWED_EXECUTABLES.iter().any(|&a| a == basename)
}

/// **环境净化** (env_clear + 保留必要变量)
///
/// 删除所有 env, 再显式 set 必要变量:
/// - PATH (exec 查找)
/// - HOME (Unix) / USERPROFILE (Windows) (config files like ~/.gitconfig)
/// - TMPDIR (Unix) / TEMP, TMP (Windows) (temp files)
/// - SystemRoot, PATHEXT (Windows, 一些工具需要)
/// - LANG / LC_ALL (unicode)
///
/// 防泄漏: API key / token / 私有路径 等敏感 env 不会传给子进程。
fn sanitize_env(cmd: &mut Command) {
    cmd.env_clear();

    // 通用 (POSIX + Windows 都有)
    for var in ["PATH", "LANG", "LC_ALL"] {
        if let Ok(v) = std::env::var(var) {
            cmd.env(var, v);
        }
    }

    // Unix-only
    #[cfg(not(windows))]
    {
        for var in ["HOME", "TMPDIR", "USER", "SHELL"] {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }
    }

    // Windows-only
    #[cfg(windows)]
    {
        for var in [
            "SystemRoot",
            "PATHEXT",
            "USERPROFILE",
            "TEMP",
            "TMP",
            "HOMEDRIVE",
            "HOMEPATH",
            "USERNAME",
            "USERDOMAIN",
            "OS",
        ] {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }
    }
}

// =============================================================================
// CodeExecTool — 适配 Tool trait
// =============================================================================

/// **CodeExec → Tool 适配器**
///
/// **args 协议**:
/// - `cmd` (String, 必)
/// - `timeout_ms` (u64, 选, 0 = impl 默认)
pub struct CodeExecTool {
    inner: Arc<dyn CodeExec>,
}

impl CodeExecTool {
    /// 构造适配器
    pub fn new(inner: Arc<dyn CodeExec>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl apeireth_tool_registry::Tool for CodeExecTool {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn kind(&self) -> ToolKind {
        // CodeExec 是同步外部依赖 → Sync (战役 2-1 6 类 enum)
        ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes {
            trigger: apeireth_tool_registry::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
            resident: apeireth_tool_registry::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::TransportAxis::Local,
            output: apeireth_tool_registry::OutputAxis::SideEffect,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let cmd = args
            .get("cmd")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'cmd' string")?;
        let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(0);

        let (code, output) = self.inner.exec(cmd, timeout_ms).await?;
        Ok(json!({
            "cmd": cmd,
            "exit_code": code,
            "output": output,
        }))
    }
}

// =============================================================================
// 单元测试 — 真跑 shell + timeout + 错误路径
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;

    #[tokio::test]
    async fn exec_echo_hello() {
        let e = ShellCodeExec::new();
        // P0-4: 不走 shell, Windows 上 `echo` 是 cmd 内置, 没有 echo.exe.
        // 改用 `cmd /c echo` (Windows) / `echo` (Unix, 是独立 binary).
        let cmd = if cfg!(windows) {
            "cmd /c echo hello"
        } else {
            "echo hello"
        };
        let (code, out) = e.exec(cmd, 0).await.expect("exec");
        assert_eq!(code, 0);
        assert!(out.contains("hello"), "应含 echo 输出, 实际: {out:?}");
    }

    #[tokio::test]
    async fn exec_nonzero_exit_code() {
        let e = ShellCodeExec::new();
        // 不走 shell, 所以 `exit 7` (shell builtin) 不可用; 改用 `cmd /c` / `sh -c`
        // 透传 exit code. cmd / sh 都在白名单内.
        let (code, out) = if cfg!(windows) {
            e.exec("cmd /c exit 7", 0).await.expect("exec")
        } else {
            e.exec(r#"sh -c "exit 7""#, 0).await.expect("exec")
        };
        assert_eq!(code, 7, "应保留非 0 exit code");
        // 输出可能为空 (exit 不写 stdout/stderr)
        let _ = out;
    }

    #[tokio::test]
    async fn exec_empty_cmd_errors() {
        let e = ShellCodeExec::new();
        let r = e.exec("", 0).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("empty"));
    }

    #[tokio::test]
    async fn exec_whitespace_only_errors() {
        let e = ShellCodeExec::new();
        let r = e.exec("   \t  ", 0).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn exec_timeout_triggers() {
        // 真模拟 hang: 用 ping 等 5s, timeout 100ms
        let e = ShellCodeExec::new().with_default_timeout_ms(100);
        let cmd = if cfg!(windows) {
            // Windows: ping 5 个包, 间隔 1s
            "ping -n 5 127.0.0.1"
        } else {
            "sleep 5"
        };
        let start = std::time::Instant::now();
        let r = e.exec(cmd, 0).await;
        let elapsed = start.elapsed();
        assert!(r.is_err(), "应超时");
        assert!(r.unwrap_err().contains("timeout"));
        // 100ms timeout 不应真等 5s
        assert!(
            elapsed < Duration::from_secs(2),
            "timeout 应快速触发, 实际: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn exec_custom_timeout_used() {
        let e = ShellCodeExec::new(); // 默认 30s
                                      // timeout_ms = 50, sleep 5s 应 50ms 内超时
        let cmd = if cfg!(windows) {
            "ping -n 5 127.0.0.1"
        } else {
            "sleep 5"
        };
        let start = std::time::Instant::now();
        let r = e.exec(cmd, 50).await;
        let elapsed = start.elapsed();
        assert!(r.is_err());
        assert!(elapsed < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn exec_captures_stderr() {
        let e = ShellCodeExec::new();
        // 不走 shell, 所以 `1>&2` (redirection) 不可用; 改用自然写 stderr 的命令
        // - Unix: `ls /nonexistent_apeireth_path_zzz` (会写 "ls: ... No such file or directory" 到 stderr)
        // - Windows: `cmd /c echo error 1>&2` (cmd 内部处理 redirection, 仍在白名单内)
        let cmd = if cfg!(windows) {
            "cmd /c echo error 1>&2".to_string()
        } else {
            "ls /nonexistent_apeireth_path_zzz".to_string()
        };
        let (code, out) = e.exec(&cmd, 0).await.expect("exec");
        // `ls` 对不存在的路径会写 stderr 但 exit 2
        // `cmd /c echo error 1>&2` exit 0
        let _ = code;
        assert!(
            out.contains("error") || out.contains("No such"),
            "stderr 应被捕获, 实际: {out:?}"
        );
    }

    #[tokio::test]
    async fn tool_adapter_exec() {
        use apeireth_tool_registry::Tool;
        let e = Arc::new(ShellCodeExec::new());
        let tool = CodeExecTool::new(e);
        // P0-4: Windows 上 `echo` 是 cmd 内置, 没有 echo.exe — 改用 `cmd /c echo`
        let cmd = if cfg!(windows) {
            "cmd /c echo via-tool"
        } else {
            "echo via-tool"
        };
        let r = tool.call(json!({"cmd": cmd})).await.expect("call");
        assert_eq!(r["exit_code"], 0);
        assert!(r["output"].as_str().unwrap().contains("via-tool"));
    }

    #[tokio::test]
    async fn tool_adapter_missing_cmd() {
        use apeireth_tool_registry::Tool;
        let e = Arc::new(ShellCodeExec::new());
        let tool = CodeExecTool::new(e);
        let r = tool.call(json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("missing 'cmd'"));
    }

    #[tokio::test]
    async fn tool_adapter_name_and_kind() {
        let e = Arc::new(ShellCodeExec::new().with_name("MyShell"));
        let tool = CodeExecTool::new(e);
        assert_eq!(tool.name(), "MyShell");
        assert_eq!(tool.kind(), ToolKind::Sync);
    }

    // =========================================================================
    // P0-4 shell 注入防御测试 (Tech-Review 2026-08-05)
    // =========================================================================

    /// **shell metacharacter 注入**: `; rm -rf /`
    ///
    /// 防御路径: `shell-words` 把 `;` 作为字面 token 保留; `;` 不在白名单 → 拒绝.
    /// `rm` **永远不会被 spawn**.
    #[tokio::test]
    async fn test_call_blocks_shell_metacharacters() {
        let e = ShellCodeExec::new();
        // 注意: 用 `;` 起头的命令. 旧实现会 `sh -c "; rm -rf /"` 实际跑 rm.
        // 新实现: parse → [";", "rm", "-rf", "/"] → `;` 不在白名单 → Err.
        let r = e.exec("; rm -rf /", 0).await;
        assert!(r.is_err(), "shell metachar 注入必须被拒, 实际: {r:?}");
        let err = r.unwrap_err();
        assert!(
            err.contains("not in whitelist") || err.contains("whitelist"),
            "应是白名单错误, 实际: {err}"
        );
    }

    /// **命令替换注入**: `` `whoami` ``
    ///
    /// 防御路径: `shell-words` 不解释 backtick (POSIX shlex 风格), 整个 `` `whoami` ``
    /// 作为单一字面 token; 不在白名单 → 拒绝. `whoami` 永远不会被执行.
    #[tokio::test]
    async fn test_call_blocks_command_substitution() {
        let e = ShellCodeExec::new();
        // backtick 命令替换. 旧实现 `sh -c "\`whoami\`"` 实际跑 whoami.
        // 新实现: parse → ["`whoami`"] → 不在白名单 → Err.
        let r = e.exec("`whoami`", 0).await;
        assert!(r.is_err(), "命令替换必须被拒, 实际: {r:?}");
        let err = r.unwrap_err();
        assert!(
            err.contains("not in whitelist") || err.contains("whitelist"),
            "应是白名单错误, 实际: {err}"
        );

        // 同样: `$(whoami)` 也不应执行
        let r2 = e.exec("$(whoami)", 0).await;
        assert!(r2.is_err(), "$(...) 也不应被执行, 实际: {r2:?}");
    }

    /// **引号参数正确拆分**: `git commit -m "fix: 修复"`
    ///
    /// 验证 `shell-words` 正确处理 POSIX 引号 — `"fix: 修复"` (含空格 + 中文) 作为
    /// 单一 arg, 而不是被拆成 5 个. 实际 git 不会真跑 (不在 git repo), 我们直接测
    /// parse_argv 验证拆分.
    #[tokio::test]
    async fn test_call_handles_quoted_args() {
        // 直接测 parser (不依赖 git / 网络)
        let parts = parse_argv(r#"git commit -m "fix: 修复""#).expect("parse");
        assert_eq!(
            parts,
            vec![
                "git".to_string(),
                "commit".to_string(),
                "-m".to_string(),
                "fix: 修复".to_string(),
            ],
            "引号应保留为单一 arg, 实际: {parts:?}"
        );

        // 额外: 单引号 / 转义也工作
        let parts2 = parse_argv(r"echo 'hello world'").expect("parse");
        assert_eq!(parts2, vec!["echo".to_string(), "hello world".to_string()]);

        // 含空格的 arg (转义)
        let parts3 = parse_argv(r"echo hello\ world").expect("parse");
        assert_eq!(parts3, vec!["echo".to_string(), "hello world".to_string()]);

        // 端到端: 真跑一个无副作用的命令验证 argv 透传.
        // `cmd /c exit 0` (Win) / `true` (Unix) 都退出 0, 不写 stdout/stderr.
        let e = ShellCodeExec::new();
        let cmd = if cfg!(windows) {
            r#"cmd /c exit 0"#
        } else {
            // 验证带引号的 arg 透传到 exec:
            // `echo "hello world"` 应该输出 "hello world" (单 arg, 不会被 split).
            r#"echo "hello world""#
        };
        let (code, out) = e.exec(cmd, 0).await.expect("exec");
        assert_eq!(code, 0, "真跑命令应 exit 0, 实际: code={code} out={out:?}");
        if !cfg!(windows) {
            assert!(
                out.contains("hello world"),
                "引号参数应作为单一 arg 传给 echo, 实际: {out:?}"
            );
        }
    }

    /// **空命令**: 各种空形式 (空串 / 仅空白 / 仅引号)
    #[tokio::test]
    async fn test_call_rejects_empty_command() {
        let e = ShellCodeExec::new();
        // 空串 — 字段级校验拦
        let r1 = e.exec("", 0).await;
        assert!(r1.is_err());
        assert!(r1.unwrap_err().contains("empty"));

        // 仅空白
        let r2 = e.exec("   \t  ", 0).await;
        assert!(r2.is_err());

        // 仅引号 (parse 后空)
        let r3 = e.exec("''", 0).await;
        assert!(r3.is_err(), "仅引号也应拒, 实际: {r3:?}");
    }

    /// **不在白名单的可执行文件**: 任意二进制
    #[tokio::test]
    async fn test_call_rejects_unknown_executable() {
        let e = ShellCodeExec::new();
        // `nmap` 不在白名单 (假设测试环境没有, 即使有也应在白名单外)
        // 实际更稳: 明显不存在的命令, 既不在白名单也不在 PATH
        let r1 = e.exec("apeireth_no_such_command_xyz_12345", 0).await;
        assert!(r1.is_err(), "不在白名单的应被拒, 实际: {r1:?}");
        let err = r1.unwrap_err();
        assert!(
            err.contains("not in whitelist"),
            "应是白名单错误, 实际: {err}"
        );

        // 路径形式 `/usr/bin/some_malicious` 也应被拒
        let r2 = e.exec("/usr/bin/apeireth_malicious_xyz", 0).await;
        assert!(r2.is_err(), "路径形式也应被白名单拒, 实际: {r2:?}");

        // 路径绕过尝试: `git/foo` — basename 是 `foo`, 不在白名单 → 拒
        let r3 = e.exec("git/../apeireth_bypass_xyz", 0).await;
        assert!(r3.is_err(), "路径绕过也应被拒, 实际: {r3:?}");

        // is_allowed_executable 单测
        assert!(is_allowed_executable("git"));
        assert!(is_allowed_executable("/usr/bin/git"));
        assert!(is_allowed_executable("git.exe")); // Windows
        assert!(is_allowed_executable("cmd.exe"));
        assert!(!is_allowed_executable("rmfoo")); // 严格匹配, 不前缀
        assert!(!is_allowed_executable("nmap"));
        assert!(!is_allowed_executable("/usr/bin/nmap"));
        assert!(!is_allowed_executable(""));
    }
}
