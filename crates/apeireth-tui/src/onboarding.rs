//! R26-3 onboarding wizard — TUI 首启 / 未配置 LLM 时走纯文本提示收集 LlmConfig
//!
//! **为什么用 stdio 而不用 ratatui UI**
//! - onboarding 只走一次（首启后写盘）, 不需要 ratatui 较多状态
//! - stdio 提示在 setup_terminal 之前 (cooked mode) 重载 stdin, 避免与 ratatui raw mode 冲突
//! - 现在不做 LLM 连接测试 (实际调用在 TUI 里发现问题, 不是这里)
//! - 调用者三次 read_line 后写 llm.json, 随后 main 继续 setup_terminal 进 ratatui
//!
//! **R27 C 方案新增 [0] 选项**:
//! - apeireth-api 本地 daemon (cargo run -p apeireth-api)
//! - 默认 base_url = http://127.0.0.1:8080/v1 (让用户确认或改)
//! - api_key 留空 (server 自己在 APEIRETH_API_KEY env 配)
//! - model 让用户填 (server 转发给真 LLM 的模型名)

use std::io::{self, BufRead, Write};

use crate::llm_config::{self, LlmConfig, Provider};

/// onboarding 选项: 5 个 preset provider + 1 个 server (R27 C)
#[derive(Debug, Clone, Copy)]
enum ProviderChoice {
    Openai,
    Anthropic,
    Deepseek,
    Ollama,
    Custom,
    ServerLocal, // [0] apeireth-api daemon
}

impl ProviderChoice {
    fn to_provider(self) -> Provider {
        match self {
            Self::Openai => Provider::Openai,
            Self::Anthropic => Provider::Anthropic,
            Self::Deepseek => Provider::Deepseek,
            Self::Ollama => Provider::Ollama,
            Self::Custom | Self::ServerLocal => Provider::Custom, // server 也是 OpenAI 协议向上
        }
    }
    /// R27 C 方案: [0] server 专用 preset, 其他走 Provider::preset()
    fn base_url_preset(self) -> Option<&'static str> {
        match self {
            Self::ServerLocal => Some("http://127.0.0.1:8080/v1"),
            _ => None,
        }
    }
    fn model_preset(self) -> &'static str {
        match self {
            Self::ServerLocal => "", // 让用户填真模型名 (server 转发用)
            _ => "",
        }
    }
}

/// 运行 onboarding (read 4 字段 + save 到盘). 错误返 Err.
pub fn run() -> io::Result<()> {
    println!();
    println!("┌────────────────────────────────────┐");
    println!("│  APEIRETH 首次启动配置 (R27 C) │");
    println!("└────────────────────────────────────┘");
    println!();
    println!("TUI 连本地 apeireth-api server, server 再转发给你的大模型。");
    println!("选 provider → 填 Base URL/Key/Model (约 30 秒)。");
    println!();
    println!("**重要**: 下面 [0]~[5] 后面的 URL/model 只是默认 (preset)。");
    println!("  后面提示 Base URL/Model 时, 直接输入你想要的值就能覆盖。回车 = 接受 preset。");
    println!("  国内大模型走快捷途径是选 [5] Custom, 填你家 service 的 OpenAI 兼容端点。");
    println!();
    println!("  [0] apeireth-api   本地 daemon (推荐 · 多端共享)");
    println!("  [1] OpenAI         OpenAI 主站 (默认 preset)");
    println!("  [2] Anthropic      Anthropic Claude (默认 preset)");
    println!("  [3] DeepSeek       OpenAI 兼容 (默认 preset)");
    println!("  [4] Ollama         本地完全未调 · 无 key (默认 preset)");
    println!("  [5] Custom         自填 OpenAI 兼容端点");
    println!();

    let stdin = io::stdin();
    let mut lock = stdin.lock();

    let choice = prompt_with_default(
        &mut lock,
        "选 provider [0-5, 默认 0]",
        |s| match s.trim() {
            "1" => Some(ProviderChoice::Openai),
            "2" => Some(ProviderChoice::Anthropic),
            "3" => Some(ProviderChoice::Deepseek),
            "4" => Some(ProviderChoice::Ollama),
            "5" => Some(ProviderChoice::Custom),
            _ => Some(ProviderChoice::ServerLocal), // 默认走 [0] (R27 C 推荐)
        },
    )?;

    let provider = choice.to_provider();

    // R27 C 方案: [0] 有专用 preset; 其他走 Provider::preset()
    let (preset_url, preset_model) = if let Some(url) = choice.base_url_preset() {
        (url.to_string(), choice.model_preset().to_string())
    } else {
        let (u, m) = provider.preset();
        (u.to_string(), m.to_string())
    };

    if matches!(choice, ProviderChoice::ServerLocal) {
        println!();
        println!("  ★ 选了 [0] apeireth-api server 模式 (本地 daemon)");
        println!("    1) 另开一个终端跑:    cargo run -p apeireth-api --release");
        println!("    2) server 端 env 配 APEIRETH_API_KEY=<你的 key>");
        println!("    3) Base URL 默认 http://127.0.0.1:8080/v1 (下面会让你确认或改)");
        println!("    4) api_key 留空 (server 自己转发)");
        println!("    5) model 填服务端真正用的模型名 (e.g. minimax-M3)");
        println!();
    }

    let base_url = prompt_with_default_str(
        &mut lock,
        &format!("Base URL [回车用 preset {preset_url}, 或输入你的]"),
        &preset_url,
    )?;

    let api_key = if matches!(choice, ProviderChoice::ServerLocal) {
        println!("  (server 模式 api_key 留空, 下面回车即可)");
        prompt_with_default_str(&mut lock, "API Key [回车留空]", "")?
    } else if provider == Provider::Ollama {
        println!("  (Ollama 本地, 不需 API key, 直接回车)");
        String::new()
    } else {
        println!("  (API Key 底下会写到 {}, 本提示不回显)", llm_display_path());
        prompt_with_default_str(&mut lock, "API Key", "")?
    };

    let model = prompt_with_default_str(
        &mut lock,
        &format!("Model [回车用 preset {preset_model}, 或输入你的]"),
        &preset_model,
    )?;

    let cfg = LlmConfig::new(provider, &base_url, &api_key, &model);
    if !cfg.is_valid() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "所有字段都不能为空, 重跑 onboarding",
        ));
    }
    llm_config::save(&cfg)?;
    println!();
    println!("✓ 配置已写到 llm.json");
    println!("  provider = {}", cfg.provider.label());
    println!("  base_url = {}", cfg.base_url);
    println!("  api_key  =              sk-*** (后 4 位)");
    println!("  model    = {}", cfg.model);
    println!();
    Ok(())
}

/// 提示一行 + 默认. 返 Result。使用者直接回车采默认。
fn prompt_with_default<F, T>(
    lock: &mut impl BufRead,
    prompt: &str,
    parse: F,
) -> io::Result<T>
where
    F: FnOnce(&str) -> Option<T>,
{
    print!("  {prompt}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    let _ = lock.read_line(&mut buf);
    let s = buf;
    parse(&s).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("解析失败: {s:?}"))
    })
}

fn prompt_with_default_str(
    lock: &mut impl BufRead,
    prompt: &str,
    default: &str,
) -> io::Result<String> {
    print!("  {prompt}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    let _ = lock.read_line(&mut buf);
    let trimmed = buf.trim();
    Ok(if trimmed.is_empty() { default.to_string() } else { trimmed.to_string() })
}

fn llm_display_path() -> String {
    llm_config::llm_config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<unresolved>".to_string())
}
