// 网关事件频道的流式读取面 — fetch 流式 SSE (带鉴权头过令牌门)。
//
// 背景 (W2-② 全 API 面令牌门之后的前端收口): EventSource 无法携带
// Authorization 头, 带令牌的客户端只能改走 fetch 流式读取才能过门; 无令牌
// (三态门未设, 兼容路径)的客户端保留 EventSource 原路径, 行为与收口前一致。
//
// 线格式解析 (createSseFrameParser) 与 EventSource 同口径: 空行分发帧、
// `data:` 多行按 \n 连接、无名帧 event 名为 'message'、无 data 的帧不发、
// 断流时未完帧丢弃不猜。帧级语义沿用 presence.ts 的帧解析约定
// (presence_state → parsePresenceStateFrame), 本层不理解任何帧载荷。
//
// 本文件零运行时依赖 (DOM/EventSource/fetch 全部惰性守卫), 可被 Node 直接
// import 测试 (tests/sse-fetch-stream.mjs)。

/** 一帧 SSE: 具名帧带事件名, 无名帧 event 恒为 'message' (与 onmessage 同口径)。 */
export interface SseFrame {
  /** 帧事件名; 无名帧为 'message'。 */
  event: string;
  /** 帧 data 体: 多行 `data:` 行按 \n 连接, 尾部换行按契约去掉。 */
  data: string;
}

export interface SseFrameParser {
  /** 喂入一段已解码文本, 返回其中已完整的帧 (顺序即到达序)。 */
  push(text: string): SseFrame[];
  /** 流结束: 未完帧作废 (与 EventSource 同口径, 不猜不补)。 */
  finish(): void;
}

/**
 * 增量 SSE 线格式解析器 (EventSource 语义逐条对齐):
 *   - 行分隔 \n / \r\n / \r; 尾部孤立 \r 留待下一块 (可能是 \r\n 的一半, 不猜);
 *   - `field: value` — 值只去掉一个前导空格;
 *   - `data` 行追加 `值 + \n`; `event` 行置帧名; 注释行 (`:` 开头) 忽略;
 *   - 空行 = 分发点: data 缓冲为空的帧不发 (含只有 event 行的帧);
 *   - 发出的 data 去掉尾部单个换行; 无名帧 event 名为 'message';
 *   - id / retry / 未知字段忽略 (帧级语义不猜)。
 */
export function createSseFrameParser(): SseFrameParser {
  let buffer = '';
  let eventName = '';
  let dataBuffer = '';

  /** 切出完整行; 尾部孤立 \r 留在 rest (等下一块判定是否 \r\n)。 */
  function takeLines(text: string): {lines: string[]; rest: string} {
    const lines: string[] = [];
    let start = 0;
    let i = 0;
    while (i < text.length) {
      const ch = text[i];
      if (ch === '\n') {
        lines.push(text.slice(start, i));
        i += 1;
        start = i;
      } else if (ch === '\r') {
        if (i + 1 >= text.length) break;
        lines.push(text.slice(start, i));
        i += text[i + 1] === '\n' ? 2 : 1;
        start = i;
      } else {
        i += 1;
      }
    }
    return {lines, rest: text.slice(start)};
  }

  function processLine(line: string, out: SseFrame[]): void {
    if (line === '') {
      if (dataBuffer !== '') {
        out.push({event: eventName || 'message', data: dataBuffer.slice(0, -1)});
      }
      eventName = '';
      dataBuffer = '';
      return;
    }
    if (line.startsWith(':')) return;
    const colon = line.indexOf(':');
    const field = colon < 0 ? line : line.slice(0, colon);
    let value = colon < 0 ? '' : line.slice(colon + 1);
    if (value.startsWith(' ')) value = value.slice(1);
    if (field === 'data') dataBuffer += `${value}\n`;
    else if (field === 'event') eventName = value;
  }

  return {
    push(text) {
      buffer += text;
      const {lines, rest} = takeLines(buffer);
      buffer = rest;
      const out: SseFrame[] = [];
      for (const line of lines) processLine(line, out);
      return out;
    },
    finish() {
      buffer = '';
      eventName = '';
      dataBuffer = '';
    },
  };
}

export interface SseStreamHandlers {
  /** 建链成功 (响应过门) — 调用方复位退避 / 清降级计时。 */
  onOpen: () => void;
  /** 到帧即送 (顺序即到达序)。 */
  onFrame: (frame: SseFrame) => void;
  /** 流断 / 失败 (含非 2xx 过门失败) — 调用方按自己的退避重连。 */
  onEnd: () => void;
}

/**
 * fetch 流式读取一条 SSE 频道 (鉴权头由调用方经 gatewayAuthHeaders() 取)。
 * 返回取消函数: 中断读取且之后不再回调。流自然读尽也回报 onEnd —— 对订阅
 * 频道而言"读尽"就是断线, 重连决策权在调用方。
 */
export function startSseFetchStream(
  url: string,
  headers: Record<string, string>,
  handlers: SseStreamHandlers,
): () => void {
  const controller = new AbortController();
  const parser = createSseFrameParser();
  let closed = false;

  void (async () => {
    try {
      const response = await fetch(url, {
        headers: {...headers, Accept: 'text/event-stream'},
        signal: controller.signal,
      });
      if (closed) return;
      if (!response.ok || !response.body) {
        handlers.onEnd();
        return;
      }
      handlers.onOpen();
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      try {
        while (true) {
          const {done, value} = await reader.read();
          if (closed) return;
          if (done) break;
          for (const frame of parser.push(decoder.decode(value, {stream: true}))) {
            handlers.onFrame(frame);
          }
        }
      } finally {
        reader.releaseLock();
      }
      parser.finish();
      if (!closed) handlers.onEnd();
    } catch {
      // fetch 拒绝 / 读取中断; 取消 (closed) 不回报。
      if (!closed) handlers.onEnd();
    }
  })();

  return () => {
    closed = true;
    controller.abort();
  };
}
