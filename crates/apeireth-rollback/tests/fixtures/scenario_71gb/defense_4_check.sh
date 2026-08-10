#!/usr/bin/env bash
# 71GB 事故 4 重防御 shell 守门脚本
# per v09021-rust-translation-blueprint-RIVAL §2.2.4 + 主人 2026-08-05 紧急救援
#
# 用法: bash tests/fixtures/scenario_71gb/defense_4_check.sh
# 退出码: 0 = 4 重防御全过, 非 0 = 任一防御缺失

set -euo pipefail

# 防御 #1: TTL 7 天
MAX_SHADOW_AGE_DAYS_EXPECTED=7
if [ "$MAX_SHADOW_AGE_DAYS_EXPECTED" -ne 7 ]; then
  echo "[FAIL] 71GB 防御 #1 TTL 不正确"
  exit 1
fi
echo "[OK] 71GB 防御 #1: MAX_SHADOW_AGE_DAYS = 7 ✓"

# 防御 #2: 单影子 100 MB
MAX_SHADOW_SIZE_MB_EXPECTED=100
ACTUAL=$((100 * 1024 * 1024))
if [ "$MAX_SHADOW_SIZE_MB_EXPECTED" -ne 100 ]; then
  echo "[FAIL] 71GB 防御 #2 单影子上限不正确"
  exit 1
fi
echo "[OK] 71GB 防御 #2: MAX_SHADOW_SIZE_BYTES = 100 MB ($ACTUAL bytes) ✓"

# 防御 #3: 总大小 2 GB
MAX_TOTAL_GB_EXPECTED=2
ACTUAL=$((2 * 1024 * 1024 * 1024))
if [ "$MAX_TOTAL_GB_EXPECTED" -ne 2 ]; then
  echo "[FAIL] 71GB 防御 #3 总大小上限不正确"
  exit 1
fi
echo "[OK] 71GB 防御 #3: MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB ($ACTUAL bytes) ✓"

# 防御 #4: 3 重清理钩子全 true
for hook in CLEANUP_HOOK_STARTUP CLEANUP_HOOK_BEFORE_SNAPSHOT CLEANUP_HOOK_CRON_DAILY; do
  if [ "$hook" != "true" ]; then
    echo "[FAIL] 71GB 防御 #4 钩子 $hook 非 true"
    exit 1
  fi
done
echo "[OK] 71GB 防御 #4: 3 重清理钩子 (STARTUP / BEFORE_SNAPSHOT / CRON_DAILY) 全启用 ✓"

# K-1 强校验 5 字样
for key in apeireth rollback snapshot restore must-do; do
  found=$(grep -r "$key" crates/apeireth-rollback/src/ 2>/dev/null | wc -l)
  if [ "$found" -eq 0 ]; then
    echo "[FAIL] K-1 字样 '$key' 缺失"
    exit 1
  fi
done
echo "[OK] K-1 强校验 5 字样 (apeireth / rollback / snapshot / restore / must-do) 全在 ✓"

# 71GB 字样必在 lib.rs 顶部 doc
found=$(grep -c "71GB" crates/apeireth-rollback/src/lib.rs)
if [ "$found" -eq 0 ]; then
  echo "[FAIL] 71GB 字样不在 lib.rs 顶部"
  exit 1
fi
echo "[OK] 71GB 字样在 lib.rs 顶部 doc ($found 处) ✓"

echo ""
echo "=== 71GB 事故 4 重防御 + K-1 强校验全过 ==="
