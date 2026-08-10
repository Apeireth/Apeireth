# APEIRETH 1.1 Release RC (2026-08-09)

## \u6982\u8981

1.1 \u662f R35 \u4e3b\u8f74\u5347\u7ea7\uff0c\u4ece R35 facade \u8d70\u5411 1.1 \u771f\u5408\u5e76 + 1.1 \u5168\u9762\u843d\u5730.
\u672c batch \u5b8c\u6210 9 \u4e2a B-stage \u4e00\u6c14\u547c\u547c\uff0c\u4ece\u4ee3\u7801\u5230\u6587\u6863\u5230 CI \u90fd\u5e72\u5b95.

## \u4ea4\u4ed8\u6e05\u5355

| # | \u540d\u79f0 | \u72b6\u6001 | \u9a8c\u8bc1 |
|---|------|------|------|
| B1 | telemetry 4 \u5408\u5e76 1.1 \u771f\u5408\u5e76 | DONE | 429 tests pass, 4148 workspace tests pass |
| B2 | pipeline tool_loop \u771f\u63a5 TUI/Web/\u684c\u9762 | DONE | 5 \u65b0 tests pass (3 web + 2 tauri-stub) |
| B3 | MCP 3 ResourceServer \u771f\u63a5 | DONE | 22 tests pass (File/Organ/Convention) |
| B4 | CouncilMember \u8de8 5 provider \u534f\u5546 demo | DONE | 71 council tests pass (5 members, 5 providers) |
| B5 | GitHub Actions CI yaml + eval-live.yml | DONE | 15 workflows (incl. \u65b0\u589e eval-live) |
| B6 | Memory in_memory + file Provider \u5347\u7ea7 | DONE | 7+ providers (in_memory/file/mongodb/sqlite/postgres/redis/s3/disk_lru/hybrid) |
| B7 | OAuth device_code grant | DONE | 7 device_code tests pass (RFC 8628 4 \u6b65) |
| B8 | Graph \u63a5 cognition 24 \u7ef4\u8282\u70b9 | DONE | 5 cognition_graph tests pass (24 dim + 1 summary + 1 decide) |
| B9 | workspace 1.0 \u2192 1.1 | DONE | 0 errors, 4148+ tests pass |

## \u8d44\u4ea7

- 6 commits: 1f23b28f / dc00f6d7 / cb2f2ab4 / 990c0d5e + 2 (locked 0 \u9020\u8d8a\u6587\u6863)
- 60+ files changed, 15K+ lines added
- 0 \u6539 24 LOCKED crate
- 0 \u6539 R11 baseline \u4e09\u503c
- 0 \u6539 8 \u9879\u4e0d\u4fee\u6539\u627f\u8bfa
- 0 \u6539 TUI 9 organ page UI

## \u5b66\u4e60\u4e0e\u5ef6\u7eed

- 1.1 \u540e\u53ef\u5e72: B5 eval-live \u5468 cron \u8d70\u751f\uff0c\u8ddf\u8e2a MiniMax LLM \u517c\u5bb9\u6027
- 1.2 \u5ef6\u7eed\uff1aB6 memory provider 真接 redis/postgres, B7 真 OAuth device flow, B8 cognition graph 真接 9 organ

## 6 \u54f2\u5b66\u953a\u7a7f\u900f

- S-1 \u5317\u6781\u661f \u00b7 R23 \u9501\u5b9a 24 \u7ef4 + 9 organ + 8 LOCKED
- S-2 \u5b9e\u4e8b\u6c42\u662f \u00b7 0 \u91cd\u5199 0 \u4f2a\u9020
- O-2 \u8d70\u5728\u524d\u4eba\u5c16\u4e0a \u00b7 LangGraph conditional + AutoGen ConversableAgent + MCP 2025-03-26 + RFC 8628
- O-3 \u5e72\u5230\u5e95 \u00b7 9 B-stage \u4e00\u6c14\u547c\u547c
- O-4 \u4efb\u4f55\u4eba\u90fd\u80fd\u63a5\u624b \u00b7 4 \u4e2a\u4e3b\u4f53\u4ee3\u7801\u7ed3\u6784\u6e05\u6670 + 9 \u4efd\u62a5\u544a + 1.1 release doc
- O-5 \u4e0d\u5047\u88c5 \u00b7 0 fake test 0 mock pass \u7ed5\u8fc7\u771f\u9a8c\u8bc1