//! Recon Suite：证据优先的静态分析与网络侦察工具。
//!
//! 共享基础设施：
//!
//! * [`evidence::Evidence`] —— 结论 + 证据 + 显式局限；
//! * [`scope::Scope`] —— fail-closed 的授权清单（路径 / 域名 / 网段）；
//! * [`platform::ToolPlatform`] —— 宿主 × 目标 × provider 的能力描述。
//!
//! # 工具清单
//!
//! | 模块 | 工具 | 能力 | 边界 |
//! |---|---|---|---|
//! | [`pack`] | `re.pack` | 字符串 / 熵 / 魔数 / 嵌入雕刻 | 静态，不执行目标 |
//! | [`binfmt`] | `re.binfmt` | ELF / PE / Mach-O / fat 结构 | 不做反汇编 |
//! | [`dotnet`] | `re.dotnet` | CLR 元数据探测 | 表枚举有限 |
//! | [`js`] | `re.js` | 词法级 JS/Electron 观察 | 非 AST |
//! | [`yara`] | `re.yara` | YARA 子集规则 | 无正则/模块/外部变量 |
//! | [`net`] | `net.recon` | TCP connect + 被动 banner | 无 raw socket |
//! | [`web`] | `web.probe` | HTTP 指纹 + 安全头 | 不跟随重定向、不读 body |
//! | [`proto`] | `net.proto` | pcap / pcapng → 流 + DNS | 不重组 TCP、不解密 |
//! | [`tls`] | `re.tls` | TLS 握手指纹 | 不读证书链 |
//!
//! 各工具模块只依赖共享基础设施，不互相依赖；注册与治理在工具面完成。
//! 能力差距一律写进 `Evidence.limitations`，不假装覆盖。

pub mod binfmt;
pub mod dotnet;
pub mod evidence;
pub mod js;
pub mod net;
pub mod pack;
pub mod platform;
pub mod regex;
pub mod scope;
pub mod yara;

pub use binfmt::{parse as parse_binary, BinFmtError, BinaryReport};
pub use dotnet::{detect as detect_dotnet, DotnetError, DotnetReport};
pub use evidence::{rfc3339_utc, Evidence, EvidenceError, EvidenceItem};
pub use js::{analyze_js, JsOptions, JsReport};
pub use net::{scan as scan_ports, NetError, PortFinding, ScanOptions, ScanReport};
pub use pack::{analyze as analyze_pack, EmbeddedRegion, ExtractedString, PackOptions, PackReport};
pub use platform::{current_host, HostOs, PlatformError, ToolPlatform};
pub use scope::{Scope, ScopeError};
pub use yara::{RuleMatch, RuleSet, YaraError, YaraReport};
