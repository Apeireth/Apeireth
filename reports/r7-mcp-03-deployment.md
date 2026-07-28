# R7-MCP-03 — HQB MCP stdio 本地部署 + PID 管理方案

任务 `2d1b0cb3-...` · mcp_integration_expert · 2026-07-22. 仅方案, 不写代码/commit. 依赖 R7-MCP-01 (7 工具 / 5 守门) + R7-MCP-02 (F4 PID 冲突落地).

## 1. stdio 本地部署

**路径**: `artifacts/hqb/` (独立, 不污染 V1074).
- `mcp.pid` — PID 单实例锁 / `mcp.log` — stdout 工具流水 / `mcp.err` — stderr 内部错误 / `audit.log` — 审计 (verdict_ignored/veto_forced/restart).

**命令**: `nohup python -m apeireth.mcp_hqb_server >artifacts/hqb/mcp.log 2>artifacts/hqb/mcp.err &`
**脚本**: `bin/start_hqb_mcp.sh` (POSIX, 主路径) + `bin/start_hqb_mcp.bat` (Windows 备用, 主人 OS).
**顺序**: ① V1074 真测 (保 `asi_snapshot.json`) → ② hqb-mcp 后台启 → ③ `search_mcp_tools({"query":"hqb"})` 验 7 工具可见.

stdio 无端口 (与 mcp-ssh-deploy/winrm 不同), 进程内 FIFO; **单进程单实例**, 重复启即 PID 冲突 → R7-MCP-02 F4.

## 2. PID 管理

```bash
P=artifacts/hqb/mcp.pid
write_pid()      { echo $$ > $P; }
alive_check()    { kill -0 $(cat $P) 2>/dev/null; }
graceful_stop()  { kill -TERM $(cat $P); wait $! 2>/dev/null; }
force_stop()     { kill -9 $(cat $P); }                  # 慎用
auto_restart()   { ! alive_check && start_hqb_mcp.sh; }
clear_pid()      { rm -f $P; }
```

**Windows**: `tasklist /FI "PID eq $(cat %P%)" /NH` 替 kill-0; `taskkill /PID xxx /T` 替 kill; 双平台脚本均覆盖.
**冲突检测**: 启前读 PID, `alive_check=OK` → 拒启 `E_ADDR_IN_USE`; 文件残留但 PID 死 → 清后启.
**watchdog**: `bin/hqb_mcp_watch.sh` 30s 轮询, 累计 3 次失败告警.

## 3. 日志方案

| 流 | 文件 | 内容 |
|---|---|---|
| stdout | `mcp.log` | 工具流水 (ts+decision_id+tool+args 摘要+result) |
| stderr | `mcp.err` | server 内部错 (V1085 veto / FK / E_xxx) |
| 审计 | `audit.log` | verdict_ignored / veto_forced / restart / pid_change |

**轮转**: 超 100MB 或 7 天 → `logrotate` (Linux) / `schtasks` (Windows). 不引 cron 守护.
**告警**: stderr 命中 `E_DANGLING_REF`/`E_TRACE_DANGLING`/`E_NO_BASELINE`/`E_BAD_ACTION` → 同步 `audit.log` + stderr 立刻可见 (主人直读 stderr 即知, 不引外部告警).
**审计格式**: `<ts> <event> <id|-> <detail>`; `grep ^veto_forced audit.log` 复审.

## 4. 启动前检查 (6)

- [ ] `artifacts/asi_snapshot.json` 在 (V1074 baseline)
- [ ] `apeireth/hqb/hqb.db` 初始化 (`hqb_meta.schema_version=0.1.0`)
- [ ] `artifacts/hqb/` 目录在 (`mkdir -p`)
- [ ] 无残留 `mcp.pid` (避免 E_ADDR_IN_USE)
- [ ] Python 路径 OK (`APEIRETH_PYTHON_PATH` 或 cd promethean)
- [ ] `python -c "import apeireth.mcp_hqb_server"` 无 ImportError

任一失败 `exit 1`, server 不启.

## 5. 优雅关闭 (6 步)

1. 收 SIGTERM (taskkill /F 前 SIGTERM)
2. 拒新调用 (`shutting_down=True` → `E_SHUTTING_DOWN`)
3. 等进行中完成 (timeout 30s, 超时 print `audit.log`)
4. flush WAL (`conn.commit()` + fsync, hqb.db 不漏)
5. 删 `mcp.pid`
6. `sys.exit(0)`

**强制**: SIGTERM 超 30s 不退 → SIGKILL (R7-MCP-02 F4 慎用); WAL 可能未 flush, 启时 `PRAGMA wal_checkpoint(TRUNCATE)` 自动恢复.

## 6. 结论

stdio + PID 文件 + 三流日志覆盖 R7-MCP-01/02 全部契约 (V1074 只读 / V1085 veto 强制 / FK CASCADE / stdio 单实例); 真实现按本方案落地即可. 验收: ≤3KB ✓ / 部署完整 / PID 命令具体 / 日志三流 / 启前 6 / 优雅 6 步 / 不写码.
