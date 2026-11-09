//! 会话隐私**声明载体**的端到端钉死 (W3 · 单元 2)。
//!
//! 覆盖面:
//!
//! 1. **零变化**: 没有声明 ⇒ 信号 `Defaulted` ⇒ 隐私闸不过滤 ⇒ 选路与「直接调
//!    `CostRouter`」逐条相同;
//! 2. **真读点**: CLI 的回合读点从**本地会话隐私档表**取声明 (跨进程可见);
//! 3. **不变量 (源级扫描)**: 隐私档字样**不得**出现在出网路径
//!    (`protocol/` / `plugin/`) ⇒ 防将来有人把密级标注塞进出网载荷。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use apeireth_core::kernel::SessionId;
use apeireth_core::routing_policy::{PrivacyLevel, PrivacySignal};

/// 本模块的 env 串行锁 (本文件内多个用例都要改 `APEIRETH_DATA_DIR`)。
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 仓根目录 (从本 crate 的 manifest 往上三层)。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// 把 `APEIRETH_DATA_DIR` 指向一个临时目录, 跑完复原。
fn with_temp_data_dir<T>(body: impl FnOnce(&Path) -> T) -> T {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let original = std::env::var("APEIRETH_DATA_DIR").ok();
    let dir = std::env::temp_dir().join(format!("apeireth-cli-privacy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("临时数据目录");
    std::env::set_var("APEIRETH_DATA_DIR", &dir);
    let result = body(&dir);
    match original {
        Some(value) => std::env::set_var("APEIRETH_DATA_DIR", value),
        None => std::env::remove_var("APEIRETH_DATA_DIR"),
    }
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// **零变化**: 没有声明 ⇒ 缺省兜底 ⇒ 闸不过滤。
#[test]
fn a_session_without_a_declaration_reads_as_absent_and_defaults_the_signal() {
    with_temp_data_dir(|_dir| {
        let session = SessionId::new();
        assert_eq!(
            apeireth_cli::session_privacy_level_for(Some(&session)).expect("本地表可读"),
            None,
            "没声明 ⇒ 读点回 None ⇒ 信号缺省兜底 ⇒ 闸不过滤"
        );
        assert_eq!(
            apeireth_cli::turn_privacy_signal_from_value(None).expect("缺省信号"),
            PrivacySignal::Defaulted,
            "缺省 ⇒ 普通档兜底"
        );
        assert_eq!(
            apeireth_cli::session_privacy_level_for(None).expect("无会话"),
            None,
            "无会话 ⇒ 无声明"
        );
    });
}

/// **真读点**: 声明落在本地表 ⇒ CLI 回合点读得到 (跨进程: 网关写、CLI 读)。
#[test]
fn the_turn_read_point_reads_the_declaration_from_the_local_session_table() {
    with_temp_data_dir(|dir| {
        let session = SessionId::new();
        let mut table = apeireth_gateway::session_settings::SessionPrivacyTable::default();
        table.set(&session, Some(PrivacyLevel::Restricted.as_str()));
        table.save_to_data_dir(dir).expect("本地表落盘");

        assert_eq!(
            apeireth_cli::session_privacy_level_for(Some(&session)).expect("本地表可读"),
            Some("restricted".to_string()),
            "声明 ⇒ 读点拿得到 ⇒ 真进选路"
        );
        assert_eq!(
            apeireth_cli::turn_privacy_signal_from_value(Some("restricted")).expect("解析"),
            PrivacySignal::Declared(PrivacyLevel::Restricted),
            "声明值 ⇒ 信号 = 显式声明档"
        );
    });
}

/// **不变量 (源级扫描)**: 隐私档字样**不得**出现在出网路径。
///
/// 出网载荷 = 规范请求 (`protocol/`) 与插件能力面 (`plugin/`)。规范请求的
/// `metadata` 会被整包写进出网 body ⇒ 隐私档字样一旦出现在那里, 就等于把
/// 密级标注送给对端 ⇒ 这里钉死「0 命中」。
#[test]
fn the_privacy_level_never_appears_in_the_outbound_request_path() {
    let root = repo_root();
    let forbidden = [
        "privacy_level",
        "PrivacyLevel",
        "PrivacySignal",
        "privacy_signal",
        "privacy_restricted",
        "privacy_confidential",
    ];
    for crate_dir in [
        "crates/foundation/protocol/src",
        "crates/foundation/plugin/src",
    ] {
        let dir = root.join(crate_dir);
        let mut stack = vec![dir];
        let mut files = Vec::new();
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).expect("目录可读") {
                let entry = entry.expect("目录项可读");
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    files.push(path);
                }
            }
        }
        assert!(!files.is_empty(), "扫描面非空: {crate_dir}");
        for file in files {
            let text = std::fs::read_to_string(&file).expect("源文件可读");
            for token in forbidden {
                assert!(
                    !text.contains(token),
                    "出网路径不得出现隐私档字样: {} 命中 {token}",
                    file.display()
                );
            }
        }
    }
}

/// **CLI `--privacy` 入口**: 声明 ⇒ 本回合的闸真的收紧; 不认识 ⇒ 明确报错。
#[test]
fn the_cli_privacy_flag_declares_the_level_for_this_turn() {
    assert_eq!(
        apeireth_cli::turn_privacy_signal(Some("restricted"), None).expect("声明"),
        PrivacySignal::Declared(PrivacyLevel::Restricted),
        "--privacy restricted ⇒ 本回合显式声明受限档"
    );
    assert_eq!(
        apeireth_cli::turn_privacy_signal(None, None).expect("缺省"),
        PrivacySignal::Defaulted,
        "不传 ⇒ 不声明 ⇒ 缺省兜底 (零行为变化)"
    );
}

#[test]
fn an_unknown_cli_privacy_value_is_refused_instead_of_being_treated_as_absent() {
    assert!(
        apeireth_cli::turn_privacy_signal(Some("secret"), None).is_err(),
        "不认识的值 ⇒ 报错 (不降级为普通档, 不当未声明)"
    );
    assert!(
        apeireth_cli::turn_privacy_signal(Some("   "), None).is_err(),
        "空白 ⇒ 报错 (显式传了空 ≠ 没人声明)"
    );
}

#[test]
fn the_stricter_of_the_flag_and_the_session_declaration_wins() {
    // 安全优先: 命令行**不能**放宽会话的机密声明。
    assert_eq!(
        apeireth_cli::turn_privacy_signal(Some("ordinary"), Some("confidential"))
            .expect("两者并存"),
        PrivacySignal::Declared(PrivacyLevel::Confidential),
        "会话声明更严 ⇒ 取更严的一档"
    );
    assert_eq!(
        apeireth_cli::turn_privacy_signal(Some("confidential"), Some("ordinary"))
            .expect("两者并存"),
        PrivacySignal::Declared(PrivacyLevel::Confidential),
        "命令行更严 ⇒ 取更严的一档"
    );
    assert_eq!(
        apeireth_cli::turn_privacy_signal(Some("restricted"), Some("restricted")).expect("同档"),
        PrivacySignal::Declared(PrivacyLevel::Restricted)
    );
}
