// 底部状态条（余光投影）—— 四指标推导纯逻辑。
//
// 定位（00-PHILOSOPHY §2：余光 = 第四个投影；gap-plan §4.3 P1）：
// glanceable 的一行，静是默认——只有异常才挣色彩（§2.4 语义色：
// 黄=待完善/红=危险；金档留给「他在场」，本条四指标无此语义，不用金）。
//
// 0 装纪律：任何数据拿不到就显式真实口径（「不可用」/「读取失败」/窗口计数），
// 禁编造总数。端点口径诚实标注：guard events 与 memory episodes 都是窗口列表，
// 端点不提供总数——满窗即显示「N+」，title 里写明窗口大小。
//
// 本模块 DOM-free、零运行时依赖，Node 单测直 import（tests/statusbar.mjs）。

/** 三态色（§2.4）：quiet=骨白灰（默认安静）/ warn=黄（待完善）/ danger=红（危险）。 */
export type StatusTone = 'quiet' | 'warn' | 'danger';

export interface StatusIndicator {
  /** 条上小字（等宽 11px / 0.28em，§6.2 状态行）。 */
  text: string;
  tone: StatusTone;
  /** 悬停完整解释（口径与原因写全，用户问「这是真的吗」答得起）。 */
  title: string;
  /** 可点 = 是入口（连接态→重连 / 进行中回合→打断 / 计数→对应视图）。 */
  clickable: boolean;
  /** 断连超 30s 的 SIM 纪律标注（§5.4：真实来源缺失显式标）。 */
  sim?: boolean;
}

// ---- ① SSE 连接态（/v1/apeireth/events 真实连接态）----

/** 输入即 presenceStore 的两个真值 + 能力门：不发明中间态。 */
export interface SseSourceInput {
  /** capabilityAvailable('activity.sse')；null = 清单未到达/运行时离线（未知≠缺席）。 */
  supported: boolean | null;
  /** presenceStore.connected —— EventSource open 真值。 */
  connected: boolean;
  /** presenceStore.simulated —— 断连 >30s（SIM_AFTER_MS）后由订阅器置位。 */
  simulated: boolean;
}

export type SseSource = 'unsupported' | 'live' | 'retrying' | 'lost';

export function sseSourceOf(input: SseSourceInput): SseSource {
  // 只有清单明确说「没有事件面」才是 unsupported；清单未知（离线/未到达）
  // 不算缺席——此时按连接事实报，离线自然落入重连中/断连（0 装：不猜能力）。
  if (input.supported === false) return 'unsupported';
  if (input.connected) return 'live';
  // 断连 30s 内 = 退避重连中；超过 = 真实来源缺失（SIM）。
  return input.simulated ? 'lost' : 'retrying';
}

export function sseIndicator(source: SseSource): StatusIndicator {
  switch (source) {
    case 'live':
      return {
        text: 'SSE 已连接',
        tone: 'quiet',
        title: '事件总线 /v1/apeireth/events 连接中。点击手动重连健康检查。',
        clickable: true,
      };
    case 'retrying':
      return {
        text: 'SSE 重连中',
        tone: 'warn',
        title: '连接断开，指数退避重连中（2s 起，封顶 30s）。点击立即重连。',
        clickable: true,
      };
    case 'lost':
      return {
        text: 'SSE 断连',
        tone: 'danger',
        sim: true,
        title: '真实来源缺失超过 30s——presence 显示为本机中性默认（SIM 纪律 §5.4）。点击立即重连。',
        clickable: true,
      };
    case 'unsupported':
      return {
        text: 'SSE 不可用',
        tone: 'quiet',
        title: '当前运行时未声明 activity.sse 能力（0 装：不假装在线）。',
        clickable: false,
      };
  }
}

// ---- ② 当前 turn 状态（本地流真值 = busy；与第 ④ 项打断同一语义）----

export function turnIndicator(busy: boolean): StatusIndicator {
  if (busy) {
    return {
      text: '回合进行中',
      tone: 'quiet', // 进行中是常态，不是异常——不挣色
      title: '点击打断——不再听他说完（后端回合仍会跑完；打断的是收听，不是他）。',
      clickable: true,
    };
  }
  return {
    text: '回合空闲',
    tone: 'quiet',
    title: '当前没有进行中的回合。',
    clickable: false,
  };
}

// ---- ③ 守卫最近事件数（/v1/safety/guard/events 窗口计数）----

export interface GuardIndicatorInput {
  supported: boolean;
  /** 最近一次拉取的事件条数；null = 拉取失败。 */
  count: number | null;
  /** 拉取窗口上限（fetchGuardEvents limit）。 */
  limit: number;
  /** 自上次查看以来是否有新事件（App 侧 lastSeen 比较得出）。 */
  hasNew: boolean;
}

export function guardIndicator(input: GuardIndicatorInput): StatusIndicator {
  if (!input.supported) {
    return {
      text: '守卫 不可用',
      tone: 'quiet',
      title: '当前运行时未声明 safety.guard.events.read（0 装：不编造计数）。',
      clickable: false,
    };
  }
  if (input.count === null) {
    return {
      text: '守卫 读取失败',
      tone: 'warn',
      title: '事件列表拉取失败——计数缺失而非为零。点击打开治理卷宗守卫 tab 自查。',
      clickable: true,
    };
  }
  const full = input.count >= input.limit;
  return {
    text: full ? `守卫 ${input.limit}+` : `守卫 ${input.count}`,
    tone: input.hasNew ? 'warn' : 'quiet',
    title: `口径：最近 ${input.limit} 条事件窗口内的计数（端点不供总数）。` +
      (input.hasNew ? '有未读新事件。' : '') +
      '点击打开治理卷宗守卫 tab。',
    clickable: true,
  };
}

// ---- ④ 记忆 episode 计数（/v1/panel/memory/episodes 窗口计数）----

export interface MemoryIndicatorInput {
  supported: boolean;
  count: number | null;
  limit: number;
}

export function memoryIndicator(input: MemoryIndicatorInput): StatusIndicator {
  if (!input.supported) {
    return {
      text: '记忆 不可用',
      tone: 'quiet',
      title: '当前运行时未声明 memory.read（0 装：不编造计数）。',
      clickable: false,
    };
  }
  if (input.count === null) {
    return {
      text: '记忆 读取失败',
      tone: 'warn',
      title: 'episode 列表拉取失败——计数缺失而非为零。点击打开记忆视图自查。',
      clickable: true,
    };
  }
  const full = input.count >= input.limit;
  return {
    text: full ? `记忆 ${input.limit}+` : `记忆 ${input.count}`,
    tone: 'quiet', // 计数大小不是异常，永远不挣色
    title: `口径：最近 ${input.limit} 条窗口内的 episode 数（端点不供总数）。点击打开记忆视图。`,
    clickable: true,
  };
}
