//! 外部工具（协议桥）主人管理面的证据: 只读列表面 / deny 落盘 / 幂等零写 /
//! undeny 摘除 / 错误即帧 / `--` 之后当位置参数 / 并发改写不丢更新 /
//! 坏配置拒开不覆盖。
//!
//! 口径: fixture 落临时目录; `list` 只读零写; `deny`/`undeny` 只改
//! `mcp-servers.json` 一处 (需重启才生效 —— 用例断言输出写明这一点)。

use std::path::{Path, PathBuf};
use std::time::Duration;

use apeireth_cli::mcp_command::{run_mcp_at, USAGE};
use apeireth_core::stored_doc;
use apeireth_tools_canonical::mcp_bridge::{
    doc_compat, McpServerConfig, McpServerSpec, McpTransportKind, MCP_SERVERS_ENV,
};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-mcp-cli-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

/// env 入口在场时宿主读的是 env (文件入口是死的) ⇒ 写面必须拒。
/// 本文件的所有用例都要「env 不在场」这个前提; 环境变量本来就不在场时
/// `remove_var` 是空操作 (不会引入竞态面)。
fn clear_env_entry() {
    std::env::remove_var(MCP_SERVERS_ENV);
}

/// 写一份服务器名单 (`mcp-servers.json`, 存储文档信封)。
fn write_servers(dir: &Path, servers: Vec<McpServerSpec>) -> PathBuf {
    let path = dir.join("mcp-servers.json");
    stored_doc::save_single(
        &path,
        &doc_compat(),
        McpServerConfig::new(servers),
        stored_doc::DEFAULT_DOC_MODE,
    )
    .expect("save server list");
    path
}

/// 读回某服务器的封禁名单。
fn deny_list_of(dir: &Path, server: &str) -> Vec<String> {
    let config = McpServerConfig::load_from_data_dir(dir)
        .expect("load")
        .expect("file present");
    config
        .servers
        .iter()
        .find(|spec| spec.name == server)
        .expect("server present")
        .deny_tools
        .clone()
}

/// 盘面快照 (相对路径 + 字节): 只读零写的比对基准。
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<fs_err::DirEntry> = fs_err::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, out);
                continue;
            }
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            out.push((rel, fs_err::read(&path).unwrap()));
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out
}

/// 只读列表面: 服务器、enabled、封禁名单、授权名单、传输方式都在。
#[test]
fn list_shows_servers_with_enabled_deny_and_allow_lists() {
    clear_env_entry();
    let dir = temp_dir("list");
    write_servers(
        &dir,
        vec![
            McpServerSpec::stdio("demo", "demo-server")
                .with_deny_tool("write_item")
                .with_auto_authorize("inspect_one"),
            McpServerSpec::stdio("idle", "idle-server").disabled(),
        ],
    );
    let output = run_mcp_at(&dir, &args(&["list"])).expect("list");
    assert!(output.contains("config source: file"), "{output}");
    assert!(output.contains("server demo enabled=true"), "{output}");
    assert!(output.contains("server idle enabled=false"), "{output}");
    assert!(output.contains("transport=stdio"), "{output}");
    assert!(output.contains("deny_tools: write_item"), "{output}");
    assert!(
        output.contains("auto_authorize_tools: inspect_one"),
        "{output}"
    );
    assert!(output.contains("not probed"), "未探测要喊破: {output}");
}

/// 空配置如实说「没有」 (不是默认服务器)。
#[test]
fn list_says_honestly_when_nothing_is_configured() {
    clear_env_entry();
    let dir = temp_dir("empty");
    let output = run_mcp_at(&dir, &args(&["list"])).expect("list");
    assert!(output.contains("config source: none"), "{output}");
    assert!(output.contains("servers: 0"), "{output}");
}

/// 只读面零写: `list` 跑完盘面零变化 (含 `--live` 的只读探测)。
#[test]
fn list_writes_nothing_to_the_disk() {
    clear_env_entry();
    let dir = temp_dir("read-only");
    write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let before = snapshot(&dir);
    run_mcp_at(&dir, &args(&["list"])).expect("list");
    run_mcp_at(&dir, &args(&["list", "--live"])).expect("list --live");
    let after = snapshot(&dir);
    assert_eq!(before.len(), after.len(), "只读面不得多写文件");
    for ((rel, bytes), (rel_after, bytes_after)) in before.iter().zip(after.iter()) {
        assert_eq!(rel, rel_after);
        assert_eq!(bytes, bytes_after, "{rel:?} 不得变");
    }
}

/// `deny` 落盘: 条目真进 `mcp-servers.json` 的 `deny_tools`, 且输出写明
/// **需重启生效** (不得声称当场生效)。
#[test]
fn deny_lands_the_tool_in_the_server_list_and_asks_for_a_restart() {
    clear_env_entry();
    let dir = temp_dir("deny");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let output = run_mcp_at(&dir, &args(&["deny", "demo.write_item"])).expect("deny");
    assert!(output.contains("tool.mcp.demo.write_item"), "{output}");
    assert!(
        output.contains("takes effect after the host restarts"),
        "必须写明需重启生效: {output}"
    );
    assert!(
        !output.contains("immediately") && !output.contains("takes effect now"),
        "不得声称当场生效: {output}"
    );
    assert_eq!(deny_list_of(&dir, "demo"), vec!["write_item".to_string()]);
    // 落点就是既有配置文件本体 (零新持久化格式)。
    assert!(path.exists());
    let raw = fs_err::read_to_string(&path).unwrap();
    assert!(raw.contains("write_item"), "条目真落文件: {raw}");
}

/// 幂等: 重复 deny 报「已封禁」, 且**一个字节都不重写** (mtime 逐字不变)。
#[test]
fn deny_is_idempotent_and_does_not_rewrite_the_file() {
    clear_env_entry();
    let dir = temp_dir("idempotent");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    run_mcp_at(&dir, &args(&["deny", "demo.lookup"])).expect("first deny");
    let bytes_after_first = fs_err::read(&path).unwrap();
    let mtime_after_first = fs_err::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let second = run_mcp_at(&dir, &args(&["deny", "demo.lookup"])).expect("second deny");
    assert!(second.contains("already denied"), "{second}");
    assert!(
        !second.contains("updated"),
        "第二次不得声称改了文件: {second}"
    );
    assert_eq!(
        bytes_after_first,
        fs_err::read(&path).unwrap(),
        "内容不得变"
    );
    assert_eq!(
        mtime_after_first,
        fs_err::metadata(&path).unwrap().modified().unwrap(),
        "幂等路径不得落盘 (mtime 不变)"
    );
    assert_eq!(deny_list_of(&dir, "demo"), vec!["lookup".to_string()]);
}

/// `undeny` 把条目摘掉 (名单里没有 ⇒ 报「没封」且零写)。
#[test]
fn undeny_removes_the_tool_from_the_server_list() {
    clear_env_entry();
    let dir = temp_dir("undeny");
    write_servers(
        &dir,
        vec![McpServerSpec::stdio("demo", "demo-server").with_deny_tool("write_item")],
    );
    let output = run_mcp_at(&dir, &args(&["undeny", "demo.write_item"])).expect("undeny");
    assert!(output.contains("tool.mcp.demo.write_item"), "{output}");
    assert!(
        output.contains("takes effect after the host restarts"),
        "解封同样需重启生效: {output}"
    );
    assert!(deny_list_of(&dir, "demo").is_empty(), "条目真被摘掉");
    let again = run_mcp_at(&dir, &args(&["undeny", "demo.write_item"])).expect("undeny again");
    assert!(again.contains("not denied"), "{again}");
}

/// 未知子动词: 错误即帧 (带用法帧), 点名动词。
#[test]
fn unknown_verb_refuses_with_the_usage_frame() {
    clear_env_entry();
    let dir = temp_dir("verb");
    let error = run_mcp_at(&dir, &args(&["verify"])).expect_err("must refuse");
    assert!(error.contains("unknown mcp command"), "{error}");
    assert!(error.contains("verify"), "点名动词: {error}");
    assert!(error.contains(USAGE), "错误即帧带用法: {error}");
}

/// 未知旗标: 指名拒 (点名旗标 + `--` 分隔提示 + 用法帧)。
#[test]
fn unknown_flag_is_named_in_the_error_frame() {
    clear_env_entry();
    let dir = temp_dir("flag");
    write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    for argv in [
        args(&["deny", "demo.lookup", "--nope"]),
        args(&["list", "--nope"]),
        args(&["deny", "demo.lookup", "--live"]),
    ] {
        let error = run_mcp_at(&dir, &argv).expect_err("must refuse");
        assert!(
            error.contains("--nope") || error.contains("--live"),
            "{error}"
        );
        assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    }
}

/// `--` 之后一律当位置参数 (不再做旗标解析)。
#[test]
fn everything_after_the_double_dash_is_a_positional() {
    clear_env_entry();
    let dir = temp_dir("dashdash");
    write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let output = run_mcp_at(&dir, &args(&["deny", "--", "demo.lookup"])).expect("deny");
    assert!(output.contains("tool.mcp.demo.lookup"), "{output}");
    assert_eq!(deny_list_of(&dir, "demo"), vec!["lookup".to_string()]);

    // `--` 之后以 `-` 开头的词也是位置参数 (不是未知旗标)。
    let error = run_mcp_at(&dir, &args(&["deny", "--", "-weird.lookup"])).expect_err("must refuse");
    assert!(
        !error.contains("unknown flag"),
        "「--」之后不得当旗标拒: {error}"
    );
    assert!(error.contains(USAGE), "错误即帧带用法: {error}");
}

/// 未知服务器 / 不合法的目标名: 一律拒, 且不写文件。
#[test]
fn unknown_server_and_malformed_targets_refuse_without_writing() {
    clear_env_entry();
    let dir = temp_dir("refuse");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let before = fs_err::read(&path).unwrap();
    let unknown = run_mcp_at(&dir, &args(&["deny", "ghost.lookup"])).expect_err("must refuse");
    assert!(unknown.contains("unknown external server"), "{unknown}");
    assert!(unknown.contains("demo"), "列出已配置服务器: {unknown}");
    for bad in ["demo", "Demo.lookup", "demo.Lookup", "demo.lookup item"] {
        let error = run_mcp_at(&dir, &args(&["deny", bad])).expect_err("must refuse");
        assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    }
    assert_eq!(before, fs_err::read(&path).unwrap(), "拒绝路径不得写文件");
}

/// 坏配置拒开: 不覆盖、不回退默认 (fail-closed)。
#[test]
fn a_broken_server_list_refuses_to_open_instead_of_being_overwritten() {
    clear_env_entry();
    let dir = temp_dir("broken");
    let path = dir.join("mcp-servers.json");
    fs_err::write(&path, "{ not json").unwrap();
    let error = run_mcp_at(&dir, &args(&["deny", "demo.lookup"])).expect_err("must refuse");
    assert!(error.contains("refused to open"), "{error}");
    assert_eq!(
        fs_err::read_to_string(&path).unwrap(),
        "{ not json",
        "坏文件不得被覆盖"
    );
    let listed = run_mcp_at(&dir, &args(&["list"])).expect_err("list must refuse too");
    assert!(listed.contains("refused to open"), "{listed}");
}

/// 并发改写不丢更新: 八个线程各拉黑一个工具, 八个条目全在 (临界区在文件锁下)。
#[test]
fn concurrent_denies_keep_every_update() {
    clear_env_entry();
    let dir = temp_dir("concurrent");
    write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let handles: Vec<_> = (0..8)
        .map(|index| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                let target = format!("demo.tool_{index}");
                run_mcp_at(&dir, &args(&["deny", &target])).expect("deny")
            })
        })
        .collect();
    for handle in handles {
        let output = handle.join().expect("thread");
        assert!(
            output.contains("takes effect after the host restarts"),
            "{output}"
        );
    }
    let list = deny_list_of(&dir, "demo");
    assert_eq!(list.len(), 8, "并发写不得丢更新: {list:?}");
    for index in 0..8 {
        assert!(
            list.contains(&format!("tool_{index}")),
            "第 {index} 条丢失: {list:?}"
        );
    }
}

/// 只读探测 (`--live`): 不可达的服务器如实记为不可达 (不假装探到了)。
#[test]
fn live_probe_reports_unreachable_servers_honestly() {
    clear_env_entry();
    let dir = temp_dir("probe");
    write_servers(
        &dir,
        vec![
            McpServerSpec::remote("demo", McpTransportKind::Http, "http://127.0.0.1:45871/mcp"),
            McpServerSpec::stdio("idle", "idle-server").disabled(),
        ],
    );
    let output = run_mcp_at(&dir, &args(&["list", "--live"])).expect("list --live");
    assert!(output.contains("unreachable"), "不可达要喊破: {output}");
    assert!(
        output.contains("not probed (server disabled)"),
        "禁用服务器不探测: {output}"
    );
    assert!(
        output.contains("probed from this process"),
        "探测快照的来源要写明: {output}"
    );
}

// ===========================================================================
// 追加: 授权白名单 (`auto_authorize_tools`) 的写面 —— `authorize` /
// `unauthorize`。
//
// 口径 (0 装诚实): 白名单回答的是「这个能力**在不在授权面里**」, **不是**
// 「会不会问人」—— 外部工具族一律**逐次人批** (`full` 预设也不豁免) ⇒
// 输出必须写明这一点, 且**不得**出现「立即放行 / 免审 / 无需审批」这类
// 措辞 (主人不得被「auto」二字误导)。
// ===========================================================================

/// 读回某服务器的授权白名单。
fn allow_list_of(dir: &Path, server: &str) -> Vec<String> {
    let config = McpServerConfig::load_from_data_dir(dir)
        .expect("load")
        .expect("file present");
    config
        .servers
        .iter()
        .find(|spec| spec.name == server)
        .expect("server present")
        .auto_authorize_tools
        .clone()
}

/// `authorize` 落盘: 条目真进 `mcp-servers.json` 的 `auto_authorize_tools`,
/// 且输出写明**需重启生效** + **白名单 ≠ 免人批** (且不出现「立即放行 /
/// 免审 / 无需审批」这类措辞)。
#[test]
fn authorize_lands_the_tool_in_the_allow_list_without_promising_a_free_pass() {
    clear_env_entry();
    let dir = temp_dir("authorize");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let output = run_mcp_at(&dir, &args(&["authorize", "demo.inspect_one"])).expect("authorize");
    assert!(output.contains("tool.mcp.demo.inspect_one"), "{output}");
    assert!(
        output.contains("takes effect after the host restarts"),
        "必须写明需重启生效: {output}"
    );
    assert!(
        output.contains("逐次人批"),
        "必须写明白名单不等于免人批: {output}"
    );
    assert!(
        output.contains("authorization surface"),
        "必须写明白名单决定的是在不在授权面里: {output}"
    );
    for forbidden in [
        "immediately",
        "without approval",
        "no approval",
        "auto-approve",
        "免审",
        "无需审批",
        "立即放行",
    ] {
        assert!(
            !output.contains(forbidden),
            "不得出现 {forbidden:?}: {output}"
        );
    }
    assert_eq!(allow_list_of(&dir, "demo"), vec!["inspect_one".to_string()]);
    // 落点就是既有配置文件本体 (零新持久化格式)。
    let raw = fs_err::read_to_string(&path).unwrap();
    assert!(raw.contains("auto_authorize_tools"), "落进既有字段: {raw}");
    assert!(raw.contains("inspect_one"), "条目真落文件: {raw}");
}

/// 幂等: 重复 authorize 报「已授权」, 且**一个字节都不重写** (mtime 逐字不变)。
#[test]
fn authorize_is_idempotent_and_does_not_rewrite_the_file() {
    clear_env_entry();
    let dir = temp_dir("authorize-idempotent");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    run_mcp_at(&dir, &args(&["authorize", "demo.inspect_one"])).expect("first authorize");
    let bytes_after_first = fs_err::read(&path).unwrap();
    let mtime_after_first = fs_err::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let second =
        run_mcp_at(&dir, &args(&["authorize", "demo.inspect_one"])).expect("second authorize");
    assert!(second.contains("already authorized"), "{second}");
    assert!(
        !second.contains("updated"),
        "第二次不得声称改了文件: {second}"
    );
    assert_eq!(
        bytes_after_first,
        fs_err::read(&path).unwrap(),
        "内容不得变"
    );
    assert_eq!(
        mtime_after_first,
        fs_err::metadata(&path).unwrap().modified().unwrap(),
        "幂等路径不得落盘 (mtime 不变)"
    );
    assert_eq!(allow_list_of(&dir, "demo"), vec!["inspect_one".to_string()]);
}

/// `unauthorize` 把条目摘掉 (名单里没有 ⇒ 报「没授权」且**零写**)。
#[test]
fn unauthorize_removes_the_tool_from_the_allow_list() {
    clear_env_entry();
    let dir = temp_dir("unauthorize");
    let path = write_servers(
        &dir,
        vec![McpServerSpec::stdio("demo", "demo-server").with_auto_authorize("inspect_one")],
    );
    let output =
        run_mcp_at(&dir, &args(&["unauthorize", "demo.inspect_one"])).expect("unauthorize");
    assert!(output.contains("tool.mcp.demo.inspect_one"), "{output}");
    assert!(
        output.contains("takes effect after the host restarts"),
        "移出授权面同样需重启生效: {output}"
    );
    assert!(
        output.contains("逐次人批"),
        "白名单面必须带「白名单 ≠ 免人批」的提醒: {output}"
    );
    assert!(allow_list_of(&dir, "demo").is_empty(), "条目真被摘掉");

    // 不在名单时: 报「没授权」且**一个字节都不写**。
    let bytes_after_first = fs_err::read(&path).unwrap();
    let mtime_after_first = fs_err::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let again =
        run_mcp_at(&dir, &args(&["unauthorize", "demo.inspect_one"])).expect("unauthorize again");
    assert!(again.contains("not authorized"), "{again}");
    assert!(
        !again.contains("updated"),
        "第二次不得声称改了文件: {again}"
    );
    assert_eq!(
        bytes_after_first,
        fs_err::read(&path).unwrap(),
        "内容不得变"
    );
    assert_eq!(
        mtime_after_first,
        fs_err::metadata(&path).unwrap().modified().unwrap(),
        "不在名单时不得落盘 (mtime 不变)"
    );
}

/// 未知动词 / 未知服务器 / 不合法的目标名: 一律拒, 且不写文件。
#[test]
fn authorize_refuses_unknown_verb_unknown_server_and_malformed_targets() {
    clear_env_entry();
    let dir = temp_dir("authorize-refuse");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let before = fs_err::read(&path).unwrap();

    let unknown_verb =
        run_mcp_at(&dir, &args(&["auto-authorize", "demo.inspect_one"])).expect_err("must refuse");
    assert!(
        unknown_verb.contains("unknown mcp command"),
        "{unknown_verb}"
    );
    assert!(
        unknown_verb.contains("auto-authorize"),
        "点名动词: {unknown_verb}"
    );
    assert!(
        unknown_verb.contains(USAGE),
        "错误即帧带用法: {unknown_verb}"
    );

    let unknown_server =
        run_mcp_at(&dir, &args(&["authorize", "ghost.inspect_one"])).expect_err("must refuse");
    assert!(
        unknown_server.contains("unknown external server"),
        "{unknown_server}"
    );
    assert!(
        unknown_server.contains("demo"),
        "列出已配置服务器: {unknown_server}"
    );

    for bad in [
        "demo",
        "Demo.inspect_one",
        "demo.Inspect_one",
        "demo.inspect one",
    ] {
        let error = run_mcp_at(&dir, &args(&["authorize", bad])).expect_err("must refuse");
        assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    }
    assert_eq!(before, fs_err::read(&path).unwrap(), "拒绝路径不得写文件");
}

/// env 入口在场时**拒写** (与 deny 同纪律): 宿主读 env ⇒ 改文件没有效果。
///
/// ⚠️ 这一条走**真进程**: env 是进程全局的, 同进程内的并行用例会互相看见
/// ⇒ 在测试进程里改 env 会污染别的用例 (既有 deny 用例也会跟着被拒) ⇒
/// 起一个真二进制, 让它的 env 说话。
#[test]
fn the_env_entry_in_the_room_makes_allow_list_edits_refuse() {
    let dir = temp_dir("authorize-env");
    let path = write_servers(&dir, vec![McpServerSpec::stdio("demo", "demo-server")]);
    let before = fs_err::read(&path).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["mcp", "authorize", "demo.inspect_one"])
        .env("APEIRETH_DATA_DIR", &dir)
        .env(
            MCP_SERVERS_ENV,
            r#"[{"name":"demo","transport":"stdio","command":"demo-server","enabled":true}]"#,
        )
        .output()
        .expect("run the cli");
    assert!(!output.status.success(), "env 在场时必须拒: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(MCP_SERVERS_ENV), "{stderr}");
    assert_eq!(before, fs_err::read(&path).unwrap(), "拒绝路径不得写文件");
}

/// 两个名单**互不干扰**: 改一个不动另一个 (同一个 spec、同一份文件)。
#[test]
fn the_deny_list_and_the_allow_list_do_not_disturb_each_other() {
    clear_env_entry();
    let dir = temp_dir("cross");
    write_servers(
        &dir,
        vec![McpServerSpec::stdio("demo", "demo-server").with_deny_tool("write_item")],
    );
    // 授权名单动一笔 ⇒ 封禁名单一个字都不动。
    run_mcp_at(&dir, &args(&["authorize", "demo.inspect_one"])).expect("authorize");
    let config = McpServerConfig::load_from_data_dir(&dir)
        .expect("load")
        .expect("file present");
    let spec = config
        .servers
        .iter()
        .find(|spec| spec.name == "demo")
        .expect("server present");
    assert_eq!(spec.auto_authorize_tools, vec!["inspect_one".to_string()]);
    assert_eq!(
        spec.deny_tools,
        vec!["write_item".to_string()],
        "封禁名单不得被授权面改动"
    );

    // 封禁名单动一笔 ⇒ 授权名单一个字都不动。
    run_mcp_at(&dir, &args(&["deny", "demo.write_item2"])).expect("deny");
    let config = McpServerConfig::load_from_data_dir(&dir)
        .expect("load")
        .expect("file present");
    let spec = config
        .servers
        .iter()
        .find(|spec| spec.name == "demo")
        .expect("server present");
    assert_eq!(
        spec.deny_tools,
        vec!["write_item".to_string(), "write_item2".to_string()]
    );
    assert_eq!(
        spec.auto_authorize_tools,
        vec!["inspect_one".to_string()],
        "授权名单不得被封禁面改动"
    );
}

/// 封禁**胜出**: 两个名单同时罩着一个工具时, 授权面救不回被封的工具 (封禁
/// 闸是单调的) ⇒ 输出必须写明这一点, 免得主人以为「授权了就等于会跑」。
#[test]
fn the_deny_list_wins_over_the_allow_list_and_the_output_says_so() {
    clear_env_entry();
    let dir = temp_dir("shadow");
    write_servers(
        &dir,
        vec![McpServerSpec::stdio("demo", "demo-server").with_deny_tool("locked_tool")],
    );
    let output = run_mcp_at(&dir, &args(&["authorize", "demo.locked_tool"])).expect("authorize");
    assert!(
        output.contains("deny list wins"),
        "必须写明封禁胜出: {output}"
    );
    assert!(output.contains("stays blocked"), "{output}");
    let config = McpServerConfig::load_from_data_dir(&dir)
        .expect("load")
        .expect("file present");
    let spec = config
        .servers
        .iter()
        .find(|spec| spec.name == "demo")
        .expect("server present");
    // 两个名单互不干扰 (授权面记下了这条) ⇒ 但封禁闸照旧罩着它。
    assert!(spec
        .auto_authorize_tools
        .contains(&"locked_tool".to_string()));
    assert!(spec.denies("locked_tool"), "封禁闸照旧罩着这个工具");

    // 反向: 封禁面看到授权名单也点名它时, 同样写明封禁胜出。
    let denied = run_mcp_at(&dir, &args(&["deny", "demo.locked_tool"])).expect("deny");
    assert!(
        denied.contains("deny list wins"),
        "反向也要写明封禁胜出: {denied}"
    );
}
