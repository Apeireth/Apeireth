#!/usr/bin/env python3
"""**能力大汇总表 · 生成器 + 守卫**（Apeireth v2）

# 用途

给主人一张**可再生**的「Apeireth 现在拥有哪些能力、每条什么状态」全景表。
表本体在 `docs/03-reference/capability-status-summary.md`，由本脚本**从机器真源重算**生成 ⇒
**手改即漂移 ⇒ `--check` 非 0 退出**。

# 真源（4 处，全部机器读）

| 真源 | 提供什么 |
|---|---|
| `crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs` | **能力 id 策展表**（生产注册面的权威清单）+ 库级保留面 + 动态族根 |
| `docs/architecture/env-registry.toml` | 每个 `APEIRETH_*` 开关的登记（default / level / evidence） |
| `docs/status/claims.yaml` | 对外承诺台账（level / default / switch_env / verify_cmd） |
| 策展表 + 登记表 + 台账 | 交叉核验「策展表有没有登记这个 id」「开关登记了没」「台账口径冲不冲突」 |

# 人工维护的部分只有一处

`CURATED`（本脚本内的策展数据：id / 工具名 / 族 / 默认 / 开关 / 证据 file:line / 四级口径）——
它来自 **2026-11-08 的实测复核**（生产装配根可达性 + 未设 env 时的默认行为），
但**每次生成都会被上面 4 处真源回核**：
① id 必须在策展表里（动态族除外）；② 开关名必须在 env 登记表里；③ 台账口径冲突要报出来；
④ **证据 file:line 必须仍指向活文件**（文件存在 + 行号在范围内）⇒ 既有面被搬走 ⇒ 立刻红。

# 用法

    python3 scripts/gen_capability_summary.py --gen      # 重算并重写生成块
    python3 scripts/gen_capability_summary.py --check     # 重算并比对（不一致 ⇒ 非 0 退出）
    python3 scripts/gen_capability_summary.py --report    # 只打印统计

# 退出码

    0 = 表与真源一致
    1 = 不一致（逐条打印差在哪 + 怎么修）
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs" / "03-reference" / "capability-status-summary.md"
TAXONOMY = ROOT / "crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs"
ENVREG = ROOT / "docs/architecture/env-registry.toml"
CLAIMS = ROOT / "docs/status/claims.yaml"

BEGIN = "<!-- BEGIN:generated -->"
END = "<!-- END:generated -->"

#: 人工策展数据（2026-11-08 实测复核；每次生成被 4 处真源回核）
CURATED = {
 "REG": [
  {
   "id": "tool.filesystem",
   "tool": "filesystem",
   "family": "Filesystem",
   "default": "ON",
   "switch": "无 env 开关（APEIRETH_DISABLE_LOCAL_READ_TOOLS=1 只撤授权，不撤注册）",
   "evidence": "装配 production.rs:531-538；默认 production_config.rs:364；组合根 cli/src/lib.rs:2261(..default())",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.search",
   "tool": "search",
   "family": "Search",
   "default": "ON",
   "switch": "同上",
   "evidence": "装配 production.rs:540-546；默认 production_config.rs:365",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.repo",
   "tool": "repo",
   "family": "Repo",
   "default": "ON",
   "switch": "同上",
   "evidence": "装配 production.rs:548-554；默认 production_config.rs:366",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.repo_map",
   "tool": "repo_map",
   "family": "Repo（子族）",
   "default": "ON",
   "switch": "无 env 开关（组合根硬写 repo_map: true）",
   "evidence": "装配 production.rs:560-566；组合根 cli/src/lib.rs:2152",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.self_status",
   "tool": "self_status",
   "family": "SelfStatus",
   "default": "ON",
   "switch": "无 env 开关（默认 true，组合根不覆盖）",
   "evidence": "装配 production.rs:852-984；默认 production_config.rs:393",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.trait_report",
   "tool": "trait_report",
   "family": "TraitReport",
   "default": "ON",
   "switch": "无 env 开关（组合根恒注入成长账落位）",
   "evidence": "装配 production.rs:721-725；组合根 cli/src/lib.rs:2210-2212",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.shell",
   "tool": "shell",
   "family": "Shell",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_SHELL=1（只认 \"1\"）",
   "evidence": "装配 production.rs:645-649；默认 production_config.rs:367(shell: None)；env cli/src/lib.rs:117,2048-2050,2157-2160",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.fetch",
   "tool": "fetch",
   "family": "Fetch",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_FETCH=1",
   "evidence": "装配 production.rs:651-662；默认 production_config.rs:368(fetch: None)；env cli/src/lib.rs:118,2051-2053,2161",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.apply_patch",
   "tool": "apply_patch",
   "family": "ApplyPatch（受控写）",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_FILE_WRITE=1 且 工作区根已注入",
   "evidence": "装配 production.rs:573-579；默认 production_config.rs:394(file_write: false)；env cli/src/lib.rs:176,626-630,2183",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.git_write",
   "tool": "git_write",
   "family": "GitWrite",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_GIT_WRITE=1 且工作区根",
   "evidence": "装配 production.rs:604-610；默认 production_config.rs:396；env cli/src/lib.rs:186,668-672,2185",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.worktree_plan",
   "tool": "worktree_plan",
   "family": "WorktreePlan",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_WORKTREE_PLAN=1 且 APEIRETH_ENABLE_FILE_WRITE=1（子依赖）且工作区根",
   "evidence": "装配 production.rs:585-591（config.worktree_plan && config.file_write）；env cli/src/lib.rs:196,686-691,2190",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.worktree_apply",
   "tool": "worktree_apply",
   "family": "WorktreeApply",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_WORKTREE_APPLY=1 且 …WORKTREE_PLAN=1 且 …FILE_WRITE=1 且工作区根",
   "evidence": "装配 production.rs:593-599；env cli/src/lib.rs:197,696-701,2191",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.skill_distill",
   "tool": "skill_distill",
   "family": "SkillDistill",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_SKILL_DISTILL=1 且 …FILE_WRITE=1 且工作区根 且 技能库目录",
   "evidence": "装配 production.rs:616-622；env cli/src/lib.rs:202,706-711,2194-2197",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.skill_search",
   "tool": "skill_search",
   "family": "SkillSearch",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_SKILL_SEARCH=1 且 技能库目录",
   "evidence": "装配 production.rs:624-633；env cli/src/lib.rs:203,714-718,2195-2197",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.image_ops",
   "tool": "image_ops",
   "family": "ImageOps",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_IMAGE_OPS=1 且 …FILE_WRITE=1 且工作区根",
   "evidence": "装配 production.rs:750-757（config.image_ops && config.file_write）；env cli/src/lib.rs:2228 + tools/src/image_ops.rs:107,2094-2098",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.doc_ops",
   "tool": "doc_ops",
   "family": "DocOps",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_DOC_OPS=1（写动作另受 …FILE_WRITE=1）",
   "evidence": "装配 production.rs:764-770；env tools/src/doc_ops.rs:131,384-388 + cli/src/lib.rs:2233",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.sheet_ops",
   "tool": "sheet_ops",
   "family": "SheetOps",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_SHEET_OPS=1（写动作另受 …FILE_WRITE=1）",
   "evidence": "装配 production.rs:778-784；env tools/src/sheet_ops.rs:142,411-415 + cli/src/lib.rs:2238",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.content_feed",
   "tool": "content_feed",
   "family": "ContentFeed",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_CONTENT_FEED=1",
   "evidence": "装配 production.rs:667-671；默认 production_config.rs:369；env cli/src/lib.rs:123,732-736,2163",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.care_schedule",
   "tool": "care_schedule",
   "family": "CareSchedule",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_CARE=1",
   "evidence": "装配 production.rs:678-682；默认 production_config.rs:370；env cli/src/lib.rs:128,741-745,2167",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.backup_export",
   "tool": "backup_export",
   "family": "BackupExport",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_BACKUP_EXPORT=1",
   "evidence": "装配 production.rs:687-691；默认 production_config.rs:372；env cli/src/lib.rs:132,749-753,1899-1902,2170",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.voice_speak",
   "tool": "voice_speak",
   "family": "VoiceSpeak",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_VOICE_SPEAK=1",
   "evidence": "装配 production.rs:697-704；默认 production_config.rs:371；env cli/src/lib.rs:138,759-763,2175",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.task_board",
   "tool": "task_board",
   "family": "TaskBoard",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_TASK_BOARD=1",
   "evidence": "装配 production.rs:711-715；默认 production_config.rs:373；env cli/src/lib.rs:142,769-773,2179",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.exposure_audit",
   "tool": "exposure_audit",
   "family": "ExposureAudit",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_EXPOSURE_AUDIT=1",
   "evidence": "装配 production.rs:730-734；默认 production_config.rs:404；env cli/src/lib.rs:148,399-418,2216",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.proposal_draft",
   "tool": "proposal_draft",
   "family": "Proposals",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_PROPOSALS=1",
   "evidence": "装配 production.rs:740-743；默认 production_config.rs:405；env tools/src/proposal.rs:69,1134-1137 + cli/src/lib.rs:2221-2223",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.education",
   "tool": "education",
   "family": "Education",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_EDUCATION=1",
   "evidence": "装配 production.rs:636-640；默认 production_config.rs:391(education: false)；env cli/src/lib.rs:3055-3061,2151",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.device_sense",
   "tool": "device_sense",
   "family": "DeviceSense",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_DEVICE_SENSE=1",
   "evidence": "装配 production.rs:791-796；默认 production_config.rs:409；env tools/src/device/sense.rs:74,794-798 + cli/src/lib.rs:2242-2244",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（真实协议族仅留插槽，实测面 = mock 适配器）"
  },
  {
   "id": "tool.weather",
   "tool": "weather",
   "family": "Weather",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_DEVICE_SENSE=1（同族同闸）",
   "evidence": "装配 production.rs:797-801；env tools/src/device/weather.rs:768-771 + cli/src/lib.rs:2245-2247",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（默认 mock 档，live 档留插槽不接）"
  },
  {
   "id": "tool.device_control",
   "tool": "device_control",
   "family": "DeviceControl",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_DEVICE_SENSE=1 且 APEIRETH_ENABLE_DEVICE_CONTROL=1（双闸）",
   "evidence": "装配 production.rs:809-815；env tools/src/device/device_control.rs:126,130,145-147,196-204 + cli/src/lib.rs:2251-2254",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（执行面 = mock 适配器）"
  },
  {
   "id": "tool.guard_watch",
   "tool": "guard_watch",
   "family": "GuardWatch",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_DEVICE_SENSE=1 且 APEIRETH_ENABLE_GUARD_WATCH=1（双闸）",
   "evidence": "装配 production.rs:822-826；env tools/src/device/guard_watch.rs:74,1105-1119 + cli/src/lib.rs:2258-2260",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.play_propose",
   "tool": "play_propose",
   "family": "BioPlay（install_bio_play）",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_BIO_PLAY=1 且工作区根",
   "evidence": "装配 production.rs:831-837 → bio_production_wiring.rs:249-267；id 常量 canonical/play_tools.rs:61,70；env play_tools.rs:58",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.play_start",
   "tool": "play_start",
   "family": "BioPlay",
   "default": "OFF",
   "switch": "同上",
   "evidence": "同上；id play_tools.rs:63,72",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.play_finish",
   "tool": "play_finish",
   "family": "BioPlay",
   "default": "OFF",
   "switch": "同上",
   "evidence": "同上；id play_tools.rs:65,74",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.play_abort",
   "tool": "play_abort",
   "family": "BioPlay",
   "default": "OFF",
   "switch": "同上",
   "evidence": "同上；id play_tools.rs:67,76",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.ritual_done",
   "tool": "ritual_done",
   "family": "仪式面（install_bio_ritual_tools）",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1 且 关怀数据目录已注入",
   "evidence": "装配 production.rs:842-847 → bio_production_wiring.rs:281-299；id canonical/ritual_tools.rs:77,82；env care_reminder.rs:255",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.ritual_skip",
   "tool": "ritual_skip",
   "family": "仪式面",
   "default": "OFF",
   "switch": "同上",
   "evidence": "同上；id ritual_tools.rs:79,84",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.mcp.<server>.<tool>（动态族，逐个外部工具生成；不存在固定 tool.mcp 能力 id）",
   "tool": "mcp:<server>:<tool>",
   "family": "Mcp",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_MCP=1（+ 服务器列表可加载）",
   "evidence": "装配 production.rs:999-1017；id 生成 tools/src/mcp_bridge/catalog.rs:137-143；族根 tools/src/mcp_bridge/family.rs:20；env cli/src/lib.rs:212,2054-2056,2200-2201",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（docs/status/claims.yaml:124：2026-11-07 起四环已在真第三方 stdio MCP 服务器端到端成立，但仍有 5 类性质未覆盖 ⇒ 整体口径不成立）"
  },
  {
   "id": "tool.recon.pack",
   "tool": "re.pack",
   "family": "Recon",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_RECON=1 且工作区根（+ APEIRETH_TOOL_SCOPE）",
   "evidence": "装配 production.rs:1023-1029；id tools/src/recon_tool.rs:33；env tools/src/recon/wiring.rs:32,48-50 + cli/src/lib.rs:2204",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.binfmt",
   "tool": "re.binfmt",
   "family": "Recon",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:278",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.yara",
   "tool": "re.yara",
   "family": "Recon",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:359",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.net",
   "tool": "re.net.recon",
   "family": "Recon（网络子族）",
   "default": "OFF",
   "switch": "同上（scope 为空 ⇒ 目标全拒）",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:462；网络子族 id 表 tools/src/recon/wiring.rs（RECON_NETWORK_TOOL_IDS）",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.dotnet",
   "tool": "re.dotnet",
   "family": "Recon",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:586",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.js",
   "tool": "re.js",
   "family": "Recon",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:694",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.pcap",
   "tool": "re.pcap",
   "family": "Recon",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:784",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.web",
   "tool": "re.web.probe",
   "family": "Recon（网络子族）",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:876",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.recon.tls",
   "tool": "re.tls",
   "family": "Recon（网络子族）",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1023-1029；id recon_tool.rs:969",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.nmap",
   "tool": "ext.nmap",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "APEIRETH_ENABLE_EXTERNAL=1",
   "evidence": "装配 production.rs:1034-1038；id tools/src/external.rs:165；env tools/src/recon/wiring.rs:34,53-55 + cli/src/lib.rs:2205",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（子进程封装；系统二进制缺失时该工具永久报错）"
  },
  {
   "id": "tool.external.exiftool",
   "tool": "ext.exiftool",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:481",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.tshark",
   "tool": "ext.tshark",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:593",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.yara",
   "tool": "ext.yara",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:730",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.binwalk",
   "tool": "ext.binwalk",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:874",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.radare2",
   "tool": "ext.radare2",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:991",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.jadx",
   "tool": "ext.jadx",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:1156",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.apktool",
   "tool": "ext.apktool",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:1278",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.ghidra",
   "tool": "ext.ghidra",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:1433",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.external.openssl",
   "tool": "ext.openssl",
   "family": "ExternalTools",
   "default": "OFF",
   "switch": "同上",
   "evidence": "production.rs:1034-1038；id external.rs:1591",
   "caliber": "IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立"
  }
 ],
 "UNREACH": [
  {
   "id": "tool.memory.retrieve",
   "tool": "（无模型面名，MemoryTool 内部能力）",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无 env 开关",
   "evidence": "id 生成 runtime-assembly/src/canonical/memory_module.rs:57；impl ToolCapability :276；全仓无 MemoryModule::new 调用（rg -n \"\\bMemoryModule\\b\" crates/ --glob '*.rs' \\",
   "caliber": "grep -v \"/tests/\" ⇒ 只命中定义处）"
  },
  {
   "id": "tool.memory.write",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上（OPERATIONS 常量 memory_module.rs:25-27）",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.memory.update",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.memory.delete",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.memory.search",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.memory.compact",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.memory.organize",
   "tool": "同上",
   "family": "MemoryModule",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.mesh.connect",
   "tool": "（MeshTool 内部能力）",
   "family": "Mesh",
   "default": "OFF",
   "switch": "无 env 开关（Host-only 配置，不读 env）",
   "evidence": "id 生成 runtime-assembly/src/mesh.rs:26；Kind::suffix :301-307；全仓 MeshModule::new 只在 tests/mesh_execution.rs",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立（docs/status/claims.yaml:1094-1110 rt.mesh_local_worker 等级 = IMPLEMENTED、default OFF，与本表一致）"
  },
  {
   "id": "tool.mesh.status",
   "tool": "同上",
   "family": "Mesh",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.mesh.exchange",
   "tool": "同上",
   "family": "Mesh",
   "default": "OFF",
   "switch": "无",
   "evidence": "同上",
   "caliber": "IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.process",
   "tool": "无（库级面，非工具）",
   "family": "Process（库级保留面）",
   "default": "不适用",
   "switch": "无",
   "evidence": "无 ToolCapability 实现（rg -c \"impl ToolCapability for\" crates/capabilities/tools/src/process/ ⇒ 0）；仅被分类器引用 permission_preset.rs:61；仓内显式登记为「未注册为工具」tool_taxonomy.rs:92-104",
   "caliber": "IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.supervisor",
   "tool": "无",
   "family": "Supervisor（库级保留面）",
   "default": "不适用",
   "switch": "无",
   "evidence": "同上（tools/src/supervisor.rs 无 ToolCapability）；tool_taxonomy.rs:97-104",
   "caliber": "IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  },
  {
   "id": "tool.std_sub_supervisor",
   "tool": "无",
   "family": "Supervisor（库级保留面）",
   "default": "不适用",
   "switch": "无",
   "evidence": "同上（tools/src/std_sub_supervisor.rs 无 ToolCapability）；tool_taxonomy.rs:95-104",
   "caliber": "IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立"
  }
 ],
 "LIBMOD": [
  [
   "tool.guardrail",
   "前置守门 / 凭据绊线（消费方，不是能力）",
   "tools/src/guardrail.rs（impl ToolCapability for ⇒ 0）"
  ],
  [
   "tool.stealth_crawler",
   "3 个纯函数，零 IO，未接线",
   "tools/src/stealth_crawler.rs（无 ToolCapability）"
  ],
  [
   "tool.spill",
   "会话级有界大文本溢出存储",
   "tools/src/spill.rs（无 ToolCapability）"
  ],
  [
   "tool.object_access",
   "对象级访问判定（E-P3）",
   "tools/src/object_access.rs（无 ToolCapability）"
  ],
  [
   "tool.redaction_bridge",
   "出境脱敏双向闸",
   "tools/src/redaction_bridge.rs（无 ToolCapability）"
  ],
  [
   "tool.write_intent",
   "shell 侧写意图词法扫描",
   "tools/src/write_intent.rs（无 ToolCapability）"
  ],
  [
   "tool.live_policy",
   "进程内共享 live 权限策略",
   "tools/src/live_policy.rs（无 ToolCapability）"
  ],
  [
   "tool.observed_gate",
   "读前观测门禁",
   "tools/src/observed_gate.rs（无 ToolCapability）"
  ],
  [
   "tool.egress",
   "受控出网策略",
   "tools/src/egress.rs（无 ToolCapability）"
  ],
  [
   "tool.escalation",
   "沙箱升级阶梯提示",
   "tools/src/escalation.rs（无 ToolCapability）"
  ],
  [
   "tool.ecosystem_health",
   "工具生态健康度内核（纯内核未接线）",
   "tools/src/ecosystem_health.rs（无 ToolCapability）"
  ],
  [
   "tool.proposal_push",
   "提案推送账",
   "tools/src/proposal_push.rs（无 ToolCapability）"
  ],
  [
   "tool.exec_pipeline",
   "五段流水线基础设施",
   "tools/src/exec_pipeline/（PipelinedCapability 是包装器，不是新能力 id）"
  ],
  [
   "tool.evidence_faces",
   "证据面",
   "tools/src/evidence_faces.rs（无 ToolCapability）"
  ],
  [
   "tool.accountability_view",
   "只读视图",
   "tools/src/accountability_view.rs（无 ToolCapability）"
  ],
  [
   "tool.memory_ledger_view",
   "只读视图",
   "tools/src/memory_ledger_view.rs（无 ToolCapability）"
  ]
 ],
 "BEHAVIOR": [
  [
   "memory_recall",
   "ON",
   "无（默认 true）",
   "production_config.rs:356；装配 production.rs:1117-1156"
  ],
  [
   "memory_writeback",
   "ON",
   "无（默认 true）",
   "production_config.rs:359；装配 production.rs:1317-1361"
  ],
  [
   "preference_recall",
   "ON",
   "无（默认 true）",
   "production_config.rs:360；装配 production.rs:1158-1168"
  ],
  [
   "self_assessment",
   "ON",
   "无（默认 true）",
   "production_config.rs:361；装配 production.rs:1192-1206"
  ],
  [
   "preference_learning",
   "ON（CLI 覆盖层默认开）",
   "APEIRETH_ENABLE_PREFERENCE_LEARNING=0 关 / APEIRETH_DISABLE_PREFERENCE_LEARNING=1 关（DISABLE 优先）",
   "cli/src/lib.rs:360-368,2137；core_memory_knob_enabled cli/src/lib.rs:272-288"
  ],
  [
   "proactive_recall",
   "ON",
   "APEIRETH_ENABLE_PROACTIVE_RECALL=0 关 / APEIRETH_DISABLE_PROACTIVE_RECALL=1 关",
   "cli/src/lib.rs:290-296,2138"
  ],
  [
   "memory_injection",
   "ON",
   "APEIRETH_ENABLE_MEMORY_INJECTION=0 关 / APEIRETH_DISABLE_MEMORY_INJECTION=1 关",
   "cli/src/lib.rs:353-358,2139"
  ],
  [
   "consolidation",
   "OFF",
   "APEIRETH_ENABLE_CONSOLIDATION=1",
   "cli/src/lib.rs:370-376,2140"
  ],
  [
   "persona_consolidation",
   "OFF",
   "APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1",
   "cli/src/lib.rs:378-386,2143"
  ],
  [
   "consolidation_hash_gate",
   "OFF",
   "APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE=1",
   "cli/src/lib.rs:388-395,2146"
  ],
  [
   "reflexion",
   "OFF",
   "APEIRETH_ENABLE_REFLEXION=1",
   "cli/src/lib.rs:420-426,2147"
  ],
  [
   "partner_bond",
   "OFF",
   "APEIRETH_ENABLE_PARTNER_BOND=1",
   "cli/src/lib.rs:3031-3037,2148"
  ],
  [
   "morphology_recall",
   "OFF",
   "APEIRETH_ENABLE_MORPHOLOGY_RECALL=1",
   "cli/src/lib.rs:3039-3045,2149"
  ],
  [
   "community_triage",
   "OFF",
   "APEIRETH_ENABLE_COMMUNITY_TRIAGE=1",
   "cli/src/lib.rs:3047-3053,2150"
  ],
  [
   "absorption_insight",
   "OFF",
   "APEIRETH_ENABLE_ABSORPTION_INSIGHT=1",
   "cli/src/lib.rs:3063-3070,2153"
  ],
  [
   "judge",
   "OFF",
   "APEIRETH_COGNITIVE_JUDGE=1",
   "cli/src/lib.rs:92,2038-2040,2131-2134"
  ],
  [
   "council",
   "OFF",
   "APEIRETH_COGNITIVE_COUNCIL=1",
   "cli/src/lib.rs:93,2041-2043,2135"
  ],
  [
   "organs",
   "OFF",
   "APEIRETH_ENABLE_ORGANS=1",
   "cli/src/lib.rs:213,2044-2046,2136"
  ],
  [
   "bio_play（4 个工具面）",
   "OFF",
   "APEIRETH_ENABLE_BIO_PLAY=1",
   "production.rs:430,831-837；nervous_switchboard.rs:51-99"
  ],
  [
   "bio_care_candidates（2 个仪式工具面）",
   "OFF",
   "APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1",
   "production.rs:431,842-847；nervous_switchboard.rs:51-99"
  ]
 ]
}

#: 四级口径词表（承 system-capabilities.md）
CALIBERS = ("NOT IMPLEMENTED", "IMPLEMENTED", "PRODUCTION WIRED", "HARDWARE VALIDATED")


def read(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except OSError:
        return ""


def taxonomy_keys() -> tuple[set[str], set[str], set[str]]:
    """策展表 id / 库级保留面 / 动态族根。"""
    src = read(TAXONOMY)
    keys = set(re.findall(r'key:\s*"([^"]+)"', src))
    lib = set(re.findall(r'"(tool\.[a-z_]+)"', (re.search(r"LIBRARY_LEVEL_FACES[^=]*=\s*&\[(.*?)\];", src, re.S) or re.match(r"()", "")).group(1) if re.search(r"LIBRARY_LEVEL_FACES[^=]*=\s*&\[(.*?)\];", src, re.S) else ""))
    dyn = set()
    m = re.search(r"DYNAMIC_FAMILY_ROOTS[^=]*=\s*&\[(.*?)\];", src, re.S)
    if m:
        dyn = set(re.findall(r'"([^"]+)"', m.group(1)))
    return keys, lib, dyn


def env_registry() -> dict:
    """env 登记表 ⇒ name → entry。"""
    try:
        import tomllib
        data = tomllib.loads(read(ENVREG))
    except Exception:
        return {}
    return {e.get("name", ""): e for e in data.get("entries", []) if isinstance(e, dict)}


def claims() -> dict:
    """claims.yaml ⇒ id → claim（轻量解析：只取顶层 claims 列表的若干字段）。"""
    try:
        import yaml
        data = yaml.safe_load(read(CLAIMS)) or {}
    except Exception:
        return {}
    out = {}
    for c in data.get("claims", []) or []:
        if isinstance(c, dict) and c.get("id"):
            out[c["id"]] = c
    return out


def evidence_alive(ev: str) -> tuple[bool, str]:
    """证据串里的 file:line 是否仍指向活文件（取第一处）。"""
    for m in re.finditer(r"([A-Za-z0-9_./-]+\.rs):(\d+)(?:-(\d+))?", ev):
        rel, ln = m.group(1), int(m.group(2))
        for cand in (rel, f"crates/engine/runtime-assembly/src/canonical/{rel}",
                     f"crates/engine/runtime/src/canonical/{rel}"):
            p = ROOT / cand
            if p.is_file():
                total = len(read(p).splitlines())
                if ln <= total:
                    return True, f"{cand}:{ln}/{total}"
                return False, f"{cand}:{ln} 超出文件行数 {total}"
    return True, ""


def switch_env(sw: str) -> str:
    """从策展串里抽出完整 env 名（若无则空）。"""
    m = re.search(r"(APEIRETH_[A-Z0-9_]+)", sw)
    return m.group(1) if m else ""


def build_table() -> tuple[str, list[str]]:
    """生成表体 + 问题清单。"""
    problems: list[str] = []
    keys, libfaces, dynroots = taxonomy_keys()
    envs = env_registry()
    claim_map = claims()

    lines: list[str] = []
    reg_ids = [r["id"] for r in CURATED["REG"]]
    dynamic = [r for r in CURATED["REG"] if "动态族" in r["id"]]

    # ---- ① 生产注册面 ----
    lines.append("### ① 生产装配根可达 · 静态能力 id")
    lines.append("")
    lines.append("| # | 能力 id | 模型面工具名 | 所属族 | 四级口径 | 默认 | 开关 | 证据（file:line，回核过） |")
    lines.append("|---:|---|---|---|---|:---:|---|---|")
    n = 0
    for r in CURATED["REG"]:
        if "动态族" in r["id"]:
            continue
        n += 1
        alive, anchor = evidence_alive(r["evidence"])
        if not alive:
            problems.append(f"{r['id']}: 证据已失效（{anchor}）⇒ 更新策展数据或修实现")
        if r["id"] not in keys:
            problems.append(f"{r['id']}: 策展表未登记该 id ⇒ 新增能力必须登记 tool_taxonomy.rs")
        sw = switch_env(r["switch"])
        if sw.startswith("APEIRETH_DISABLE_"):
            sw = ""  # 逃生门：登记表里的 default 描述的是「门本身」，不是能力默认档
        if sw and sw in envs:
            reg_default = str(envs[sw].get("default", "")).lower()
            want = r["default"].strip().lower()
            if reg_default in ("on", "off") and want in ("on", "off") and reg_default != want:
                problems.append(
                    f"{r['id']}: 默认档冲突（策展={want} · env 登记表 {sw}.default={reg_default}）⇒ 以登记表为准并更新策展数据"
                )
        cal = "PRODUCTION WIRED" if "PRODUCTION WIRED" in r["caliber"] else r["caliber"]
        lines.append(
            f"| {n} | `{r['id']}` | {r['tool']} | {r['family']} | {cal} | {r['default']} | {r['switch']} | {r['evidence']} |"
        )
    for r in dynamic:
        lines.append(
            f"| — | **{r['id']}** | {r['tool']} | {r['family']} | {r['caliber']} | {r['default']} | {r['switch']} | {r['evidence']} |"
        )

    # ---- ② 生产不可达 ----
    lines.append("")
    lines.append("### ② 有能力 id 形态，但生产装配根**不可达**")
    lines.append("")
    lines.append("| # | 能力 id | 模型面工具名 | 所属族 | 四级口径 | 默认 | 证据（file:line） | 为什么不可达 |")
    lines.append("|---:|---|---|---|---|:---:|---|---|")
    for i, r in enumerate(CURATED["UNREACH"], 1):
        alive, anchor = evidence_alive(r["evidence"])
        if not alive:
            problems.append(f"{r['id']}: 证据已失效（{anchor}）")
        why = ("库级保留面（**无 `ToolCapability` 实现**）"
               if r["id"] in libfaces else "构造点全仓只在 `tests/` 被调")
        if r["id"] in keys and r["id"] not in libfaces:
            problems.append(
                f"{r['id']}: **已进策展表** ⇒ 它现在算「生产注册面」⇒ 不可达判定已过期 ⇒ 必须重判"
            )
        lines.append(
            f"| {i} | `{r['id']}` | {r['tool']} | {r['family']} | {r['caliber']} | {r['default']} | {r['evidence']} | {why} |"
        )

    # ---- ③ 库级模块面（无能力 id）----
    lines.append("")
    lines.append("### ③ 库级模块面（**有实现、无能力 id ⇒ 不是能力**）")
    lines.append("")
    lines.append("| 库级面 | 用途 | 证据 |")
    lines.append("|---|---|---|")
    for r in CURATED["LIBMOD"]:
        alive, anchor = evidence_alive(r[2])
        if not alive:
            problems.append(f"{r[0]}: 证据已失效（{anchor}）")
        lines.append(f"| `{r[0]}` | {r[1]} | {r[2]} |")

    # ---- ④ 认知/行为模块族（无 tool.* id）----
    lines.append("")
    lines.append("### ④ 认知 / 行为模块族（**不是 `tool.*` 能力 id**，登记在 `tool.self_status` 名册里）")
    lines.append("")
    lines.append("| 模块 | 默认 | 开关 | 证据 |")
    lines.append("|:---:|:---:|---|---|")
    for r in CURATED["BEHAVIOR"]:
        alive, anchor = evidence_alive(r[3])
        if not alive:
            problems.append(f"{r[0]}: 证据已失效（{anchor}）")
        lines.append(f"| `{r[0]}` | **{r[1]}** | {r[2]} | {r[3]} |")

    # ---- ⑤ 台账交叉核验 ----
    conflicts = []
    for cid, c in claim_map.items():
        lvl = str(c.get("level", ""))
        if lvl and lvl not in CALIBERS:
            conflicts.append(f"{cid}: 台账等级 `{lvl}` 不在四级口径词表")
    lines.append("")
    lines.append("### ⑤ 对外承诺台账（`docs/status/claims.yaml`）")
    lines.append("")
    dist = {}
    for c in claim_map.values():
        dist[str(c.get("level", "?"))] = dist.get(str(c.get("level", "?")), 0) + 1
    lines.append("| 等级 | 条数 |")
    lines.append("|---|---:|")
    for k in sorted(dist, key=lambda x: -dist[x]):
        lines.append(f"| {k} | {dist[k]} |")
    lines.append(f"| **合计** | **{len(claim_map)}** |")
    if conflicts:
        lines.append("")
        lines.append("> ⚠️ **台账口径冲突**：")
        for c in conflicts:
            lines.append(f"> - {c}")
        problems.extend(conflicts)

    # ---- ⑥ 统计 ----
    reg_static = len([r for r in CURATED["REG"] if "动态族" not in r["id"]])
    on = len([r for r in CURATED["REG"] if r["default"].strip().upper() == "ON" and "动态族" not in r["id"]])
    hw = sum(1 for c in claim_map.values() if str(c.get("level")) == "HARDWARE VALIDATED")
    lines.append("")
    lines.append("### ⑥ 统计块")
    lines.append("")
    lines.append("| 指标 | 值 |")
    lines.append("|---|---:|")
    lines.append(f"| 生产可达静态能力 id | **{reg_static}** |")
    lines.append(f"| 动态能力族 | **1**（`tool.mcp.<server>.<tool>`，成员数随运行期发现而变） |")
    lines.append(f"| 默认 ON | **{on}** |")
    lines.append(f"| 默认 OFF | **{reg_static - on}** |")
    lines.append(f"| 有 id 但生产不可达 | **{len(CURATED['UNREACH'])}** |")
    lines.append(f"| 无能力 id 的库级模块面 | **{len(CURATED['LIBMOD'])}** |")
    lines.append(f"| 认知 / 行为模块族 | **{len(CURATED['BEHAVIOR'])}** |")
    lines.append(f"| 对外承诺（claims） | **{len(claim_map)}** |")
    lines.append(f"| **HARDWARE VALIDATED 达成** | **{hw}** |")
    lines.append(f"| 策展表条目（`TOOL_TAXONOMY`） | **{len(keys)}** |")

    return "\n".join(lines), problems


def render() -> str:
    body, problems = build_table()
    return body, problems


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--gen", action="store_true", help="重算并重写生成块")
    ap.add_argument("--check", action="store_true", help="重算并比对")
    ap.add_argument("--report", action="store_true", help="只打印统计")
    args = ap.parse_args()

    body, problems = render()

    if args.report:
        print(body)
        return 0

    if args.gen:
        if not PAGE.exists():
            print("页面不存在 ⇒ 先建骨架（手写段）再 --gen", file=sys.stderr)
            return 1
        text = read(PAGE)
        if BEGIN not in text or END not in text:
            print("页面缺生成块标记 ⇒ 加 <!-- BEGIN:generated --> / <!-- END:generated -->", file=sys.stderr)
            return 1
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        PAGE.write_text(f"{head}{BEGIN}\n{body}\n{END}{tail}", encoding="utf-8")
        print(f"[capability-summary] 已重写生成块（{len(body.splitlines())} 行）")
        if problems:
            print(f"[capability-summary] ⚠️ {len(problems)} 条真源问题：")
            for p in problems:
                print(f"  - {p}")
            return 1
        return 0

    # --check
    if not PAGE.exists():
        print("页面不存在", file=sys.stderr)
        return 1
    text = read(PAGE)
    if BEGIN not in text or END not in text:
        print("页面缺生成块标记", file=sys.stderr)
        return 1
    _, rest = text.split(BEGIN, 1)
    cur, _ = rest.split(END, 1)
    if cur.strip() != body.strip():
        print("[capability-summary] ❌ FAIL：生成块与真源不一致 ⇒ 跑 --gen")
        import difflib
        diff = list(difflib.unified_diff(cur.strip().splitlines(), body.strip().splitlines(),
                                         "页面", "真源", lineterm="", n=1))
        for line in diff[:40]:
            print("  " + line)
        return 1
    if problems:
        print(f"[capability-summary] ❌ FAIL：{len(problems)} 条真源问题")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("[capability-summary] ✅ PASS（表与真源一致）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
