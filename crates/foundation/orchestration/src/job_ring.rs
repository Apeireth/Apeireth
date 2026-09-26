//! 任务输出环 [`JobRing`]: 工具执行长任务的过程输出有界保留 + 双游标读取。
//!
//! # 问题
//!
//! 长任务的过程输出若无界累加, 长跑进程的内存随任务时长单调膨胀; 若读取
//! 只能整段拷贝并共享一个读取位置, 观察方与消费方会互相挪动彼此的进度。
//! 本模块把"过程输出流"组织成按**绝对字节偏移**寻址的有界环, 并把读取拆成
//! 两条互不干扰的游标:
//!
//! - **消费游标** ([`JobRing::consumer_cursor`]): 模型消费侧的读取进度, 只由
//!   [`JobRing::consume`] 推进;
//! - **观察游标** (调用方各自持有的绝对偏移): [`JobRing::read_at`] 按偏移读取,
//!   **绝不推进消费游标**, 观察者之间也互不干扰。
//!
//! # 有界与省略记账
//!
//! 环的总留存量受 `capacity_bytes` 上限约束 (可配)。超限先丢最旧的整段;
//! 若仅剩一段仍超限 (单段本身就超过上限), 丢该段最旧前缀、保住最新尾部。
//! 每次淘汰都置 [`JobRing::is_lossy`] 标志并累计省略计数
//! ([`JobRing::omitted_chunks`] / [`JobRing::omitted_bytes`]): 省略是显式
//! 事实, 不做静默降级。读取请求落在被淘汰区间时, 返回的 [`RingSlice`] 以
//! `omitted_before` 明示该处内容已被丢弃。
//!
//! # 分工边界 (与相邻机制语义互补, 互不替代)
//!
//! - **任务身份与所有权 / 完成通知** (见 [`crate::job_board`]): 本模块只管
//!   输出留存与读取; 结算、等待唤醒、取消语义在 job_board。
//! - **溢出落盘** (见 [`crate::context_budget::SpillWriter`]): 环管"过程输出流"
//!   (任务执行期间的增量输出, 有界、可续读), spill 管"单次超长内容" (单条
//!   内容超预算时整篇落盘取回), 两者互补不冲突, 互不替代。
//! - **任务硬超时** (见 [`crate::subagent_llm::SUBAGENT_DEFAULT_TIMEOUT_MS`]):
//!   环 + 完成通知先行落地解决"过程输出可见 + 结算即时唤醒"; 硬超时保留为
//!   最终兜底中止, 分工不重叠。

use std::collections::VecDeque;

/// 一段连续的过程输出, 按绝对字节偏移组织。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    offset: u64,
    bytes: Vec<u8>,
}

impl Chunk {
    /// 组织一段从绝对字节偏移 `offset` 开始的输出。
    pub fn new(offset: u64, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            offset,
            bytes: bytes.into(),
        }
    }

    /// 本段首字节的绝对字节偏移。
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// 本段内容。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// 本段末字节之后的绝对字节偏移。
    pub fn end(&self) -> u64 {
        self.offset + self.bytes.len() as u64
    }
}

/// 一次读取的结果切片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingSlice {
    /// 本次返回的首字节绝对字节偏移。
    pub start: u64,
    /// 本次返回的内容。
    pub bytes: Vec<u8>,
    /// 请求偏移早于环内留存起点时为 `true`: 该处内容已被有界淘汰。
    pub omitted_before: bool,
}

impl RingSlice {
    /// 返回的字节数。
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// 是否读到空内容。
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// 环的留存与游标快照 (只读观察用)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingStats {
    /// 环内留存起点的绝对字节偏移。
    pub base_offset: u64,
    /// 已产出末字节之后的绝对字节偏移。
    pub end_offset: u64,
    /// 消费游标 (绝对字节偏移)。
    pub consumer_cursor: u64,
    /// 当前留存量 (字节)。
    pub retained_bytes: u64,
    /// 是否发生过有界淘汰。
    pub lossy: bool,
    /// 被整段丢弃的段数。
    pub omitted_chunks: u64,
    /// 被省略的字节数 (整段丢弃 + 单段前缀截弃)。
    pub omitted_bytes: u64,
}

/// 有界输出环: `Chunk` 队列按绝对字节偏移组织, 双游标读取。
///
/// 本类型不做内部加锁; 并发共享由上层 (如 [`crate::job_board::JobBoard`])
/// 的互斥边界保证序列化正确。
#[derive(Debug)]
pub struct JobRing {
    chunks: VecDeque<Chunk>,
    capacity_bytes: usize,
    retained_bytes: u64,
    end_offset: u64,
    consumer_cursor: u64,
    lossy: bool,
    omitted_chunks: u64,
    omitted_bytes: u64,
}

impl JobRing {
    /// 新建空环: 过程输出总留存量不超过 `capacity_bytes`。
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            chunks: VecDeque::new(),
            capacity_bytes,
            retained_bytes: 0,
            end_offset: 0,
            consumer_cursor: 0,
            lossy: false,
            omitted_chunks: 0,
            omitted_bytes: 0,
        }
    }

    /// 总留存量上限 (字节)。
    pub fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// 追加一段过程输出, 返回该段的起始绝对字节偏移。
    ///
    /// 追加后若总量超限, 先丢最旧整段; 仅剩一段仍超限时截弃该段最旧前缀。
    pub fn push(&mut self, bytes: &[u8]) -> u64 {
        let offset = self.end_offset;
        if !bytes.is_empty() {
            self.end_offset += bytes.len() as u64;
            self.retained_bytes += bytes.len() as u64;
            self.chunks.push_back(Chunk::new(offset, bytes.to_vec()));
            self.enforce_capacity();
        }
        offset
    }

    /// 观察读取: 从绝对字节偏移 `offset` 读到当前产出末尾。
    ///
    /// **绝不推进消费游标**; 多个观察者各持偏移、互不干扰。
    pub fn read_at(&self, offset: u64) -> RingSlice {
        self.slice_from(offset, usize::MAX)
    }

    /// 消费读取: 从消费游标读至多 `max_bytes` 字节, 并把消费游标推进到
    /// 返回内容的末尾。
    pub fn consume(&mut self, max_bytes: usize) -> RingSlice {
        let slice = self.slice_from(self.consumer_cursor, max_bytes);
        self.consumer_cursor = slice.start + slice.bytes.len() as u64;
        slice
    }

    /// 消费游标 (绝对字节偏移) —— 只由 [`JobRing::consume`] 推进。
    pub fn consumer_cursor(&self) -> u64 {
        self.consumer_cursor
    }

    /// 环内留存起点的绝对字节偏移。
    pub fn base_offset(&self) -> u64 {
        self.chunks
            .front()
            .map_or(self.end_offset, |chunk| chunk.offset)
    }

    /// 已产出末字节之后的绝对字节偏移。
    pub fn end_offset(&self) -> u64 {
        self.end_offset
    }

    /// 当前留存量 (字节)。
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// 是否发生过有界淘汰。
    pub fn is_lossy(&self) -> bool {
        self.lossy
    }

    /// 被整段丢弃的段数。
    pub fn omitted_chunks(&self) -> u64 {
        self.omitted_chunks
    }

    /// 被省略的字节数。
    pub fn omitted_bytes(&self) -> u64 {
        self.omitted_bytes
    }

    /// 留存的段数 (有界, 不随任务时长增长)。
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// 留存与游标快照。
    pub fn stats(&self) -> RingStats {
        RingStats {
            base_offset: self.base_offset(),
            end_offset: self.end_offset,
            consumer_cursor: self.consumer_cursor,
            retained_bytes: self.retained_bytes,
            lossy: self.lossy,
            omitted_chunks: self.omitted_chunks,
            omitted_bytes: self.omitted_bytes,
        }
    }

    /// 从绝对字节偏移读至多 `max_bytes` 字节 (不触碰任何游标)。
    fn slice_from(&self, offset: u64, max_bytes: usize) -> RingSlice {
        let base = self.base_offset();
        let start = offset.clamp(base, self.end_offset);
        let omitted_before = offset < base;
        let mut bytes = Vec::new();
        for chunk in &self.chunks {
            if chunk.end() <= start {
                continue;
            }
            let from = (start.saturating_sub(chunk.offset)) as usize;
            let want = max_bytes - bytes.len();
            let take = want.min(chunk.bytes.len() - from);
            bytes.extend_from_slice(&chunk.bytes[from..from + take]);
            if bytes.len() >= max_bytes {
                break;
            }
        }
        RingSlice {
            start,
            bytes,
            omitted_before,
        }
    }

    /// 有界淘汰: 总量超限时丢最旧 (整段优先, 单段超限截前缀), 记省略账。
    fn enforce_capacity(&mut self) {
        while self.retained_bytes > self.capacity_bytes as u64 {
            match self.chunks.len() {
                0 => break,
                1 => {
                    let over = (self.retained_bytes - self.capacity_bytes as u64) as usize;
                    let Some(front) = self.chunks.front_mut() else {
                        break;
                    };
                    let over = over.min(front.bytes.len());
                    front.bytes.drain(..over);
                    front.offset += over as u64;
                    self.retained_bytes -= over as u64;
                    self.omitted_bytes += over as u64;
                    self.lossy = true;
                    if self
                        .chunks
                        .front()
                        .is_some_and(|chunk| chunk.bytes.is_empty())
                    {
                        self.chunks.pop_front();
                    }
                }
                _ => {
                    let Some(old) = self.chunks.pop_front() else {
                        break;
                    };
                    let len = old.bytes.len() as u64;
                    self.retained_bytes -= len;
                    self.omitted_chunks += 1;
                    self.omitted_bytes += len;
                    self.lossy = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod ring_smoke_tests {
    use super::*;

    #[test]
    fn push_and_read_round_trip_keeps_absolute_offsets() {
        let mut ring = JobRing::new(64);
        assert_eq!(ring.push(b"abc"), 0);
        assert_eq!(ring.push(b"de"), 3);
        assert_eq!(ring.end_offset(), 5);
        assert_eq!(ring.read_at(0).bytes, b"abcde");
    }
}
