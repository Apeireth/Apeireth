//! 能力名册投影面 —— 把**实际用于装配的**配置投影成开关全表。
//!
//! 从 [`super::self_status_source`] 移出（该文件恰在尺寸棘轮上限 2064 行）。
//! 导出路径保持不变：`canonical::roster_from_config` 与 crate 根的
//! `apeireth_runtime_assembly::roster_from_config` 逐字不变。

use apeireth_tools_canonical::CapabilitySwitch;

use super::production::ProductionModulesConfig;

/// 能力名册投影: 把**实际用于装配的** [`ProductionModulesConfig`] 与注入槽
/// 存在性投影成开关全表 (真实生效值, 非配置文本), 外加组装根补充行
/// (例如授权层的 `local_read_tools` 旋钮)。
/// 补行批 (2026-11-04): 4 类「装配后才存在」的族 —— 侦察 / 外部适配器 / 游戏练习场 / 仪式完成; 后两族键名带 `_registered` = **已注册 ≠ 可调用** (治理无 grant, 调用恒拒)。
///
/// 本地文件工具行按**实际注册条件**取值 (配置开启且工作区根已注入才算生效),
/// 名册不复述"配置说开"而注册没成的事实。同一口径延伸到挂在别的槽上的
/// 开关 (开关生效链审计续): 记忆固化挂在记忆写入模块、检索深度自适应与
/// 图社区分诊挂在记忆召回模块、前瞻召回挂在记忆召回模块、记忆注入挂在
/// 记忆协调器 —— 上游槽缺席时下游开关静默失效, 名册一律照实报 false。
/// `self_tuning` / `typed_recall` / `retrieval_tuning` 三个注入槽参数为接线事实,
/// 本函数再 AND 上游槽 (自学习挂记忆召回, 类型化召回挂记忆协调器, 检索调权
/// 挂记忆召回), 与配置行同口径。
///
/// 外部工具 (协议桥) 两行的自报语义 (2026-11-05 批, 如实): `mcp` 行 = **实际
/// 登记进运行时能力表的外部工具件数 ≥ 1**（真接线口径: 发现 → 登记 → 族授权 +
/// 逐次人批 → 派发）; 未接线 / 发现失败 / 无服务器 ⇒ 恒 false（fail-closed:
/// 不宣称未证实的工具面）。**旧口径消灭**: 「服务器列表已加载」不再占用 `mcp`
/// 行, 改由 `mcp_bridge` 行承担（桥面已装配 = 配置面事实, 与件数无关）。
/// 件数在**启动期接线之后**才有 ⇒ 装配根在自述**调用时**现投影（纯函数不变）。
///
/// **纯函数口径** (2026-11-05 批): 本函数**只读 config 与入参, 不读进程 env**
/// (同输入必同输出)。两族生物总闸 (`bio_play` / `bio_care_candidates`) 已折进
/// [`ProductionModulesConfig`], 由组装根从**同一张开关表**解析后回填 (env 读取
/// 收敛到装配根单入口); 解析失败时组装根硬失败, 默认 config 两总闸皆关 ⇒ fail-closed。
pub fn roster_from_config(
    config: &ProductionModulesConfig,
    workspace_root_present: bool,
    self_tuning: bool,
    retrieval_tuning: bool,
    typed_recall: bool,
    extras: &[CapabilitySwitch],
) -> Vec<CapabilitySwitch> {
    // 生物开关 (ADR-0009 D3 单入口): 两族总闸值取自 config (装配根回填, 本函数不读
    // env ⇒ 纯函数); 仪式面装配条件 = 总闸开 **且** 关怀数据目录已注入。
    let ritual_registered = config.bio_care_candidates && config.care_schedule.is_some();
    let mut roster = vec![
        CapabilitySwitch::new("filesystem", config.filesystem && workspace_root_present),
        CapabilitySwitch::new("search", config.search && workspace_root_present),
        CapabilitySwitch::new("repo", config.repo && workspace_root_present),
        CapabilitySwitch::new("shell", config.shell.is_some()),
        CapabilitySwitch::new("fetch", config.fetch.is_some()),
        // 内容流接入 (`tool.content_feed`, J1): 配置注入即注册; 治理 RequireApproval 级。
        CapabilitySwitch::new("content_feed", config.content_feed.is_some()),
        // V3-a 主动关怀规则面: 数据目录已注入即注册。
        CapabilitySwitch::new("care_schedule", config.care_schedule.is_some()),
        // G5 备份导出 (`tool.backup_export`): 数据源配置已注入即注册。
        CapabilitySwitch::new("backup_export", config.backup.is_some()),
        // V3-b 通用嗓发声 (`tool.voice_speak`): 发声配置已注入即注册。
        CapabilitySwitch::new("voice_speak", config.voice_speak.is_some()),
        // R3 情绪→语气微调读取面随发声配置注入 (每次发声现读合成器现值); 没带读取面 = 未接线 ⇒ false。
        CapabilitySwitch::new(
            "voice_emotion_tune",
            config
                .voice_speak
                .as_ref()
                .is_some_and(|voice| voice.emotion_reader.is_some()),
        ),
        // V6-a 任务簿 (`tool.task_board`): 任务队列已注入即注册 (与工具面同队同账)。
        CapabilitySwitch::new("task_board", config.task_board.is_some()),
        // 性格周报 (`tool.trait_report`): 成长账落位已注入即注册 (与工具面同账)。
        CapabilitySwitch::new("trait_report", config.trait_report.is_some()),
        // V5-b 暴露面体检 (`tool.exposure_audit`): 体检配置已注入即注册。
        CapabilitySwitch::new("exposure_audit", config.exposure_audit.is_some()),
        // 提案面 (`tool.proposal_draft`): 提案账落位已注入即注册 (默认关 fail-closed)。
        CapabilitySwitch::new("proposals", config.proposals.is_some()),
        // 受控文件写入 (`tool.apply_patch`): 配置开启且工作区根已注入才算生效; 自动放行子档依赖主开关。
        CapabilitySwitch::new("file_write", config.file_write && workspace_root_present),
        CapabilitySwitch::new(
            "file_write_auto_pass",
            config.file_write_auto_pass && config.file_write && workspace_root_present,
        ),
        // 创作工具面 W1 · 图像操作 (`tool.image_ops`): 开关 + `file_write` 唯一写总闸 + 工作区根齐备才生效。
        CapabilitySwitch::new(
            "image_ops",
            config.image_ops && config.file_write && workspace_root_present,
        ),
        // 创作工具面 W2 · 文档操作 (`tool.doc_ops`): 配置落位即注册 (读面不背写总闸; 写动作受 `file_write` 约束)。
        CapabilitySwitch::new("doc_ops", config.doc_ops.is_some()),
        // 创作工具面 K3 · 表格操作 (`tool.sheet_ops`): 同 `doc_ops` 口径。
        CapabilitySwitch::new("sheet_ops", config.sheet_ops.is_some()),
        // K 轴 L1 在场感知 (`tool.device_sense`): 感知配置已注入即注册 (双闸默认关 fail-closed)。
        CapabilitySwitch::new("device_sense", config.device_sense.is_some()),
        // K 轴 L1 天气读面 (`tool.weather`): 天气配置已注入即注册 (与 device_sense 同族一闸)。
        CapabilitySwitch::new("weather", config.weather.is_some()),
        // K 轴 L3/L4 受控执行 (`tool.device_control`): 执行配置已注入即注册 (双闸默认关 fail-closed)。
        CapabilitySwitch::new("device_control", config.device_control.is_some()),
        // K 轴 L5 看家骨架 (`tool.guard_watch`): 守护配置已注入即注册 (双闸默认关 fail-closed)。
        CapabilitySwitch::new("guard_watch", config.guard_watch.is_some()),
        // git 写分级 (`tool.git_write`): 配置开启且工作区根已注入才算生效; 「自动放行 L1」子档依赖主开关 (L2 出网永远人批)。
        CapabilitySwitch::new("git_write", config.git_write && workspace_root_present),
        CapabilitySwitch::new(
            "git_write_auto_pass_l1",
            config.git_write_auto_pass_l1 && config.git_write && workspace_root_present,
        ),
        // 干活能力轴 P2 (worktree 两级): 开关 + `file_write` 唯一写总闸 + 工作区根齐备才生效。
        CapabilitySwitch::new(
            "worktree_plan",
            config.worktree_plan && config.file_write && workspace_root_present,
        ),
        CapabilitySwitch::new(
            "worktree_apply",
            config.worktree_apply && config.file_write && workspace_root_present,
        ),
        // 外部工具 (协议桥) 两行, 口径分离:
        // `mcp` = **实际登记进运行时能力表的外部工具件数 ≥ 1**（真接线口径:
        // 发现 → 登记 → 族授权 + 逐次人批 → 派发）—— 未接线 / 发现失败 / 无
        // 服务器 ⇒ 恒 false（fail-closed: 不宣称未证实的工具面）; 开关关也 false。
        // `mcp_bridge` = **桥面已装配**（配置面事实: 开关开且服务器列表已加载）
        // —— 与工具件数无关, 不冒充「外部工具面可用」。
        CapabilitySwitch::new("mcp", config.mcp && config.mcp_registered_tools > 0),
        CapabilitySwitch::new("mcp_bridge", config.mcp_bridge),
        // 书→技能沉淀 (I1): 写面按实际注册条件取值 (开关 + `file_write` 唯一写
        // 总闸 + 工作区根 + 技能库目录齐备才算生效, fail-closed); 只读检索按
        // (开关 + 技能库目录) 取值, 不依赖写总闸。
        CapabilitySwitch::new(
            "skill_distill",
            config.skill_distill
                && config.file_write
                && workspace_root_present
                && config.skills_dir.is_some(),
        ),
        CapabilitySwitch::new(
            "skill_search",
            config.skill_search && config.skills_dir.is_some(),
        ),
        CapabilitySwitch::new("education", config.education),
        CapabilitySwitch::new("memory_recall", config.memory_recall),
        CapabilitySwitch::new("memory_writeback", config.memory_writeback),
        CapabilitySwitch::new(
            "memory_injection",
            config.memory_injection && (config.memory_recall || config.memory_writeback),
        ),
        CapabilitySwitch::new(
            "proactive_recall",
            config.proactive_recall.is_some() && config.memory_recall,
        ),
        // 类型化召回候选只走记忆协调器 (记忆召回或记忆写入在场), 同口径。
        CapabilitySwitch::new(
            "typed_recall",
            typed_recall && (config.memory_recall || config.memory_writeback),
        ),
        CapabilitySwitch::new("preference_recall", config.preference_recall),
        CapabilitySwitch::new("preference_learning", config.preference_learning),
        // 记忆固化挂在记忆写入模块 (`with_consolidation`): 写入模块缺席时
        // 静默失效, 名册照实 false (按实际注册条件取值)。
        CapabilitySwitch::new(
            "consolidation",
            config.consolidation && config.memory_writeback,
        ),
        // V4-a 人格提取步挂在记忆写入模块的整编触发点 (`with_persona_layer`):
        // 人格层落库面缺席由组装根折进生效视图, 这里再 AND 上游槽 (写入模块 +
        // 整编触发点) —— 上游槽缺席时静默失效, 名册照实 false。
        CapabilitySwitch::new(
            "persona_consolidation",
            config.persona_consolidation && config.consolidation && config.memory_writeback,
        ),
        // V4-b 散列增量门挂在记忆写入模块的整编触发点 (`with_ingest_gate`):
        // 散列门落库面缺席由组装根折进生效视图, 这里再 AND 上游槽 (写入模块 +
        // 整编触发点) —— 上游槽缺席时静默失效, 名册照实 false。
        CapabilitySwitch::new(
            "consolidation_hash_gate",
            config.consolidation_hash_gate && config.consolidation && config.memory_writeback,
        ),
        CapabilitySwitch::new("reflexion", config.reflexion),
        CapabilitySwitch::new("self_assessment", config.self_assessment),
        CapabilitySwitch::new("judge", config.judge.enabled),
        CapabilitySwitch::new("council", config.council),
        CapabilitySwitch::new("organs", config.organs),
        CapabilitySwitch::new("partner_bond", config.partner_bond),
        // 检索深度自适应挂在记忆召回模块 (`with_morphology_recall`), 同口径。
        CapabilitySwitch::new(
            "morphology_recall",
            config.morphology_recall && config.memory_recall,
        ),
        // 图社区分诊挂在记忆召回模块 (`with_community_triage`), 同口径
        // (图谱槽在否由组装根折进生效视图)。
        CapabilitySwitch::new(
            "community_triage",
            config.community_triage && config.memory_recall,
        ),
        CapabilitySwitch::new("absorption_insight", config.absorption_insight),
        // 自学习信号只走记忆召回模块 (`with_self_tuning`): 召回缺席时接线
        // 无处生效, 名册照实 false (接线事实 AND 上游槽)。
        CapabilitySwitch::new("self_tuning", self_tuning && config.memory_recall),
        // V4-c 检索调权 (`APEIRETH_ENABLE_RETRIEVAL_TUNING`, 默认关 fail-closed)
        // 挂记忆召回模块 (`with_retrieval_tuning`): 召回缺席时注入回执面无处
        // 供给, 名册照实 false (接线事实 AND 上游槽)。
        CapabilitySwitch::new("retrieval_tuning", retrieval_tuning && config.memory_recall),
        // 补行批 (2026-11-04): 4 类「装配后才存在」的族 —— 侦察 (`tool.recon.*`,
        // 9 件) 与外部适配器 (`tool.external.*`, 10 件) 按**实际注册条件**取值;
        // 游戏练习场 (`tool.play_*`, 4 件) 与仪式完成 (`tool.ritual_done/skip`,
        // 2 件) 是**已注册 ≠ 可调用** (治理面无 grant, 模型调用恒拒), 名册只报
        // 注册事实, 不冒充可调 (键名带 `_registered` 后缀即此义)。
        CapabilitySwitch::new("recon", config.recon.is_some() && workspace_root_present),
        CapabilitySwitch::new("external", config.external.is_some()),
        CapabilitySwitch::new("play_registered", config.bio_play && workspace_root_present),
        CapabilitySwitch::new("ritual_registered", ritual_registered),
    ];
    roster.extend(extras.iter().cloned());
    roster.sort_by(|a, b| a.name.cmp(&b.name));
    roster
}
