// 治理卷宗 — 展示格式化小助手（DOM-free，Node 可直测）
// 等宽字体场景专用：时刻 / 相对时间 / 短 id。不写「暂无数据」式措辞。

/** HH:MM:SS 本地时刻（zh-CN 24h）。 */
export function formatClockMs(ts: number): string {
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit', second: '2-digit'});
}

/** MM-DD HH:MM 短日期时刻（跨年/跨天列表行用）。 */
export function formatDateTimeMs(ts: number): string {
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return '—';
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  const hm = d.toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'});
  if (sameDay) return hm;
  return `${d.toLocaleDateString('zh-CN', {month: '2-digit', day: '2-digit'})} ${hm}`;
}

/** 相对时间：刚刚 / N 秒前 / N 分钟前 / N 小时前 / N 天前。 */
export function formatRelativeMs(ts: number, nowMs: number): string {
  const diffSec = Math.floor((nowMs - ts) / 1000);
  if (diffSec < 0) return '刚刚';
  if (diffSec < 5) return '刚刚';
  if (diffSec < 60) return `${diffSec} 秒前`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin} 分钟前`;
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour} 小时前`;
  return `${Math.floor(diffHour / 24)} 天前`;
}

/** 长 id 截断：前 head 位 + …（uuid/trace_id 列表行用）。 */
export function shortId(id: string, head = 8): string {
  if (id.length <= head) return id;
  return `${id.slice(0, head)}…`;
}

/** ms 时长 → 人话（1234 → 1.23s；86 → 86ms）。 */
export function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}
