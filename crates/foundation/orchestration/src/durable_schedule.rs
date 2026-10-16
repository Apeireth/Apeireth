//! 持久调度 (周期任务 + 补发纪律): [`ScheduleStore`] 的触发、补发与写序契约。
//!
//! # 三型触发
//!
//! - [`ScheduleTrigger::After`] — 一次性: 到绝对时刻触发一次, 随后完成;
//! - [`ScheduleTrigger::Every`] — 固定间隔: 自锚点起每隔 `every_ms` 一个发生时点;
//! - [`ScheduleTrigger::Cron`] — cron 受限子集 (固定时刻重复):
//!   [`CronSubset::Daily`] 每日固定时分 / [`CronSubset::Weekly`] 每周固定星期时分。
//!
//! ## cron 子集范围声明
//!
//! 受限子集只支持「固定时刻 / 固定间隔」形态: 每日固定时分、每周固定星期时分,
//! 以及固定间隔。**不支持** 通配符 / 列表 / 范围 / 步进 / 秒字段 / `@` 简写 ——
//! 表达式形态的解析不在本模块范围。时间语义: 固定时刻按 `utc_offset_minutes`
//! 的挂钟时间计算 (偏移是常量, 无夏令时规则), 一天恒为 86 400 000 毫秒,
//! 星期取 0=周日 .. 6=周六。
//!
//! # 补发纪律 (重启不风暴)
//!
//! 错过的发生时点只补**最近一次**: [`ScheduleStore::collect_due`] 每趟每个调度
//! 至多投递一条 —— 即便期间错过了 N 个发生时点, 也只投最近的那个, 更早的一律
//! 丢弃。重启后的首趟 `collect_due` 同样只补最近一次, 不会把停机期间的全部
//! 发生时点排山倒海补发出去。
//!
//! # 写序契约 (管理写与投递写共一 FIFO 队列)
//!
//! 管理写 (新建 / 全记录 compare-and-set 更新 / 删除 / 暂停恢复) 与投递写
//! (发生记账 / 执行完成记账) 共用**一条 FIFO 队列**: [`ScheduleStore::enqueue`]
//! 按入队序派号, [`ScheduleStore::drain`] 严格按号应用, 每写一次原子落盘。
//! 更新是**全记录 compare-and-set**: 期望记录与存储记录不一致时返回
//! [`ScheduleError::Conflict`] (code [`SCHEDULE_CONFLICT_CODE`]) 且不覆盖。
//!
//! # 诚实声明 (崩溃可重复投递)
//!
//! 一次定时任务的「发生时点入队」([`ScheduleStore::collect_due`] 的投递写) 与
//! 「任务执行完成记账」([`ScheduleStore::acknowledge_execution`]) 是**两次独立的
//! 持久写**, 不在一个事务里。两次写之间崩溃 = 执行完成没有记账, 重启后
//! [`ScheduleStore::unacknowledged_deliveries`] 仍持有该发生时点, 同一发生时点
//! 会再次投递 —— 交付语义是 **at-least-once**, 执行方必须按
//! (schedule_id, occurrence_ms) 幂等去重。补发纪律约束的是「算发生时点不风暴」,
//! 与这里的重复投递是两件不同的事, 二者同时成立。
//!
//! 写路径一律走已完结的 `apeireth_core::storage_atomic` 两档原子写 (只消费):
//! 本模块全部落盘用持久档 [`storage_atomic::write_atomic_durable`], 并在
//! [`storage_atomic::with_file_lock`] 下串行化读改写 (跨线程/跨进程同序)。
//!
//! # 存储形态 (选依赖最少)
//!
//! 持久档是**整档 JSON 快照 + 原子替换** (读档即全量, 不引 SQLite、不引新依赖):
//! 档内 `write_seq` 跨重启单调, 记录按 id 定序, 同一写序列得到同一档。
//! compare-and-set 的比较单位是整条记录 ([`ScheduleRecord`] 全字段逐项相等)。

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use apeireth_core::storage_atomic::{self, DEFAULT_FILE_MODE};

/// compare-and-set 更新冲突的稳定 code: 存储记录与期望记录不一致, 不覆盖。
pub const SCHEDULE_CONFLICT_CODE: &str = "schedule_conflict";

/// 一天的毫秒数 (固定时刻计算的日长; 偏移是常量, 无夏令时规则)。
const DAY_MS: i64 = 86_400_000;

// ============================================================================
// 错误
// ============================================================================

/// 调度域错误的稳定 code 族 (供日志/界面归类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleErrorCode {
    /// 全记录 compare-and-set 冲突 (code [`SCHEDULE_CONFLICT_CODE`])。
    Conflict,
    /// 调度不存在。
    NotFound,
    /// 同 id 调度已存在。
    DuplicateId,
    /// 触发配置非法。
    InvalidTrigger,
    /// 入参不合法。
    InvalidInput,
    /// 持久档读写失败。
    Io,
    /// 持久档内容无法解析。
    Corrupt,
}

impl ScheduleErrorCode {
    /// 稳定 code 字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Conflict => SCHEDULE_CONFLICT_CODE,
            Self::NotFound => "schedule_not_found",
            Self::DuplicateId => "schedule_duplicate_id",
            Self::InvalidTrigger => "schedule_invalid_trigger",
            Self::InvalidInput => "schedule_invalid_input",
            Self::Io => "schedule_io",
            Self::Corrupt => "schedule_corrupt",
        }
    }
}

impl std::fmt::Display for ScheduleErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 调度域的失败; 每个变体都归属 [`ScheduleErrorCode`] 的一个 code。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// 全记录 compare-and-set 冲突: 期望记录与存储记录不一致, 未覆盖。
    Conflict {
        /// 被更新的调度 id。
        id: String,
        /// 期望记录携带的版本号 (与存储版本不符即冲突)。
        expected_revision: u64,
    },
    /// 调度不存在。
    NotFound {
        /// 未找到的调度 id。
        id: String,
    },
    /// 同 id 调度已存在。
    DuplicateId {
        /// 冲突的调度 id。
        id: String,
    },
    /// 触发配置非法 (间隔非正 / 时分越界 / 星期越界)。
    InvalidTrigger {
        /// 人读原因。
        reason: String,
    },
    /// 入参不合法。
    InvalidInput {
        /// 人读原因。
        reason: String,
    },
    /// 持久档读写失败。
    Io {
        /// 失败的操作。
        operation: &'static str,
        /// 人读原因。
        reason: String,
    },
    /// 持久档内容无法解析。
    Corrupt {
        /// 人读原因。
        reason: String,
    },
}

impl ScheduleError {
    /// 错误归属 code。
    pub const fn code(&self) -> ScheduleErrorCode {
        match self {
            Self::Conflict { .. } => ScheduleErrorCode::Conflict,
            Self::NotFound { .. } => ScheduleErrorCode::NotFound,
            Self::DuplicateId { .. } => ScheduleErrorCode::DuplicateId,
            Self::InvalidTrigger { .. } => ScheduleErrorCode::InvalidTrigger,
            Self::InvalidInput { .. } => ScheduleErrorCode::InvalidInput,
            Self::Io { .. } => ScheduleErrorCode::Io,
            Self::Corrupt { .. } => ScheduleErrorCode::Corrupt,
        }
    }
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict {
                id,
                expected_revision,
            } => write!(
                f,
                "schedule {id}: compare-and-set conflict at revision {expected_revision}"
            ),
            Self::NotFound { id } => write!(f, "schedule {id}: not found"),
            Self::DuplicateId { id } => write!(f, "schedule {id}: already exists"),
            Self::InvalidTrigger { reason } => write!(f, "invalid schedule trigger: {reason}"),
            Self::InvalidInput { reason } => write!(f, "invalid schedule input: {reason}"),
            Self::Io { operation, reason } => write!(f, "schedule {operation}: {reason}"),
            Self::Corrupt { reason } => write!(f, "schedule store corrupt: {reason}"),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// 调度域 result 别名。
pub type ScheduleResult<T> = Result<T, ScheduleError>;

// ============================================================================
// 触发
// ============================================================================

/// cron 受限子集: 只支持固定时刻重复触发 (范围见模块文档)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CronSubset {
    /// 每日固定时分 (挂钟时间 = UTC + `utc_offset_minutes`)。
    Daily {
        /// 时 (0..=23)。
        hour: u8,
        /// 分 (0..=59)。
        minute: u8,
        /// 相对 UTC 的固定偏移分钟 (可负; 无夏令时规则)。
        utc_offset_minutes: i32,
    },
    /// 每周固定星期时分; `weekday` 0=周日 .. 6=周六。
    Weekly {
        /// 星期 (0=周日 .. 6=周六)。
        weekday: u8,
        /// 时 (0..=23)。
        hour: u8,
        /// 分 (0..=59)。
        minute: u8,
        /// 相对 UTC 的固定偏移分钟 (可负; 无夏令时规则)。
        utc_offset_minutes: i32,
    },
}

/// 三型触发: 一次性 / 固定间隔 / cron 受限子集 (固定时刻)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleTrigger {
    /// 一次性: 到绝对时刻触发一次, 随后完成。
    After {
        /// 触发的绝对时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 固定间隔: 自 `anchor_ms` 起每隔 `every_ms` 一个发生时点 (锚点本身是第一个)。
    Every {
        /// 间隔毫秒数 (必须 > 0)。
        every_ms: i64,
        /// 锚点时刻 (epoch 毫秒)。
        anchor_ms: i64,
    },
    /// cron 受限子集: 固定时刻重复触发。
    Cron {
        /// 受限子集形态。
        spec: CronSubset,
    },
}

impl ScheduleTrigger {
    /// 触发配置过闸: 间隔必须为正, 时分/星期必须在范围内。
    pub fn validate(&self) -> ScheduleResult<()> {
        match self {
            Self::After { .. } => Ok(()),
            Self::Every { every_ms, .. } => {
                if *every_ms <= 0 {
                    Err(ScheduleError::InvalidTrigger {
                        reason: format!("every_ms must be > 0, got {every_ms}"),
                    })
                } else {
                    Ok(())
                }
            }
            Self::Cron { spec } => match spec {
                CronSubset::Daily {
                    hour,
                    minute,
                    utc_offset_minutes,
                } => validate_fixed_time(*hour, *minute, *utc_offset_minutes),
                CronSubset::Weekly {
                    weekday,
                    hour,
                    minute,
                    utc_offset_minutes,
                } => {
                    if *weekday > 6 {
                        return Err(ScheduleError::InvalidTrigger {
                            reason: format!("weekday must be 0..=6, got {weekday}"),
                        });
                    }
                    validate_fixed_time(*hour, *minute, *utc_offset_minutes)
                }
            },
        }
    }

    /// 不晚于 `now_ms` 的最近一个发生时点 (没有则为空)。
    ///
    /// 补发纪律的取值口: 每趟只认这一个时点, 更早的错过一律丢弃。
    pub fn latest_occurrence_at_or_before(&self, now_ms: i64) -> Option<i64> {
        match self {
            Self::After { at_ms } => (*at_ms <= now_ms).then_some(*at_ms),
            Self::Every {
                every_ms,
                anchor_ms,
            } => {
                if now_ms < *anchor_ms {
                    return None;
                }
                Some(*anchor_ms + (now_ms - *anchor_ms).div_euclid(*every_ms) * *every_ms)
            }
            Self::Cron { spec } => match spec {
                CronSubset::Daily {
                    hour,
                    minute,
                    utc_offset_minutes,
                } => {
                    let target = fixed_time_target_ms(*hour, *minute);
                    fixed_time_latest(now_ms, target, *utc_offset_minutes, None)
                }
                CronSubset::Weekly {
                    weekday,
                    hour,
                    minute,
                    utc_offset_minutes,
                } => {
                    let target = fixed_time_target_ms(*hour, *minute);
                    fixed_time_latest(
                        now_ms,
                        target,
                        *utc_offset_minutes,
                        Some(i64::from(*weekday)),
                    )
                }
            },
        }
    }

    /// 严格晚于 `after_ms` 的下一个发生时点 (没有则为空)。
    pub fn next_occurrence_after(&self, after_ms: i64) -> Option<i64> {
        match self {
            Self::After { at_ms } => (*at_ms > after_ms).then_some(*at_ms),
            Self::Every {
                every_ms,
                anchor_ms,
            } => {
                if after_ms < *anchor_ms {
                    return Some(*anchor_ms);
                }
                Some(*anchor_ms + ((after_ms - *anchor_ms).div_euclid(*every_ms) + 1) * *every_ms)
            }
            Self::Cron { spec } => match spec {
                CronSubset::Daily {
                    hour,
                    minute,
                    utc_offset_minutes,
                } => {
                    let target = fixed_time_target_ms(*hour, *minute);
                    fixed_time_next(after_ms, target, *utc_offset_minutes, None)
                }
                CronSubset::Weekly {
                    weekday,
                    hour,
                    minute,
                    utc_offset_minutes,
                } => {
                    let target = fixed_time_target_ms(*hour, *minute);
                    fixed_time_next(
                        after_ms,
                        target,
                        *utc_offset_minutes,
                        Some(i64::from(*weekday)),
                    )
                }
            },
        }
    }
}

/// 固定时刻的日内毫秒 (时*3600 + 分*60) * 1000。
fn fixed_time_target_ms(hour: u8, minute: u8) -> i64 {
    i64::from(hour) * 3_600_000 + i64::from(minute) * 60_000
}

fn validate_fixed_time(hour: u8, minute: u8, utc_offset_minutes: i32) -> ScheduleResult<()> {
    if hour > 23 {
        return Err(ScheduleError::InvalidTrigger {
            reason: format!("hour must be 0..=23, got {hour}"),
        });
    }
    if minute > 59 {
        return Err(ScheduleError::InvalidTrigger {
            reason: format!("minute must be 0..=59, got {minute}"),
        });
    }
    let offset_minutes = i64::from(utc_offset_minutes);
    if !(-24 * 60..=24 * 60).contains(&offset_minutes) {
        return Err(ScheduleError::InvalidTrigger {
            reason: format!("utc_offset_minutes must be -1440..=1440, got {utc_offset_minutes}"),
        });
    }
    Ok(())
}

/// 挂钟日序 → 星期: 0=周日 .. 6=周六 (日序 0 是 1970-01-01, 周四)。
fn weekday_of_day(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

/// 固定时刻: 不晚于 `now_ms` 的最近发生时点; `weekday` 为空 = 每日。
fn fixed_time_latest(
    now_ms: i64,
    target_tod_ms: i64,
    utc_offset_minutes: i32,
    weekday: Option<i64>,
) -> Option<i64> {
    let off_ms = i64::from(utc_offset_minutes) * 60_000;
    let local = now_ms.checked_add(off_ms)?;
    let day = local.div_euclid(DAY_MS);
    let tod = local.rem_euclid(DAY_MS);
    let back = match weekday {
        None => i64::from(tod < target_tod_ms),
        Some(target) => {
            let mut gap = (weekday_of_day(day) - target).rem_euclid(7);
            if gap == 0 && tod < target_tod_ms {
                gap = 7;
            }
            gap
        }
    };
    let occ_local = day
        .checked_sub(back)?
        .checked_mul(DAY_MS)?
        .checked_add(target_tod_ms)?;
    occ_local.checked_sub(off_ms)
}

/// 固定时刻: 严格晚于 `after_ms` 的下一个发生时点; `weekday` 为空 = 每日。
fn fixed_time_next(
    after_ms: i64,
    target_tod_ms: i64,
    utc_offset_minutes: i32,
    weekday: Option<i64>,
) -> Option<i64> {
    let off_ms = i64::from(utc_offset_minutes) * 60_000;
    let local = after_ms.checked_add(off_ms)?;
    let day = local.div_euclid(DAY_MS);
    let tod = local.rem_euclid(DAY_MS);
    let fwd = match weekday {
        None => i64::from(tod >= target_tod_ms),
        Some(target) => {
            let mut gap = (target - weekday_of_day(day)).rem_euclid(7);
            if gap == 0 && tod >= target_tod_ms {
                gap = 7;
            }
            gap
        }
    };
    let occ_local = day
        .checked_add(fwd)?
        .checked_mul(DAY_MS)?
        .checked_add(target_tod_ms)?;
    occ_local.checked_sub(off_ms)
}

// ============================================================================
// 记录 / 写
// ============================================================================

/// 调度状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleStatus {
    /// 在触发面上: 到点可投递。
    Active,
    /// 暂停: 不投递; 恢复后按补发纪律至多补最近一次。
    Paused,
    /// 已完成: 一次性触发已投递, 不再产生发生时点。
    Completed,
}

/// 一条持久调度记录 (更新 = 全记录 compare-and-set 的比较单位)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecord {
    /// 调度 id (全店唯一)。
    pub id: String,
    /// 版本号: 每次写入 +1, compare-and-set 的游标。
    pub revision: u64,
    /// 三型触发之一。
    pub trigger: ScheduleTrigger,
    /// 状态。
    pub status: ScheduleStatus,
    /// 随投递携带的不透明载荷。
    pub payload: String,
    /// 创建时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 最近一次写入时刻 (epoch 毫秒)。
    pub updated_at_ms: i64,
    /// 最近一次已记账的发生时点 (补发纪律的游标)。
    pub last_fired_at_ms: Option<i64>,
    /// 累计投递次数。
    pub fire_count: u64,
}

/// 一次调度投递 (携带发生时点身份, 供执行方幂等去重)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleDelivery {
    /// 来源调度 id。
    pub schedule_id: String,
    /// 发生时点 (epoch 毫秒): (schedule_id, occurrence_ms) 是幂等去重键。
    pub occurrence_ms: i64,
    /// 记账投递的时刻 (epoch 毫秒)。
    pub fired_at_ms: i64,
    /// 随投递携带的不透明载荷。
    pub payload: String,
}

/// 写的种类: 管理写与投递写共用同一 FIFO 队列, 种类只用于观测归类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleWriteKind {
    /// 管理写: 新建。
    Create,
    /// 管理写: 全记录 compare-and-set 更新。
    Replace,
    /// 管理写: 删除。
    Delete,
    /// 管理写: 暂停/恢复。
    SetStatus,
    /// 投递写: 发生时点入队 (记账一次投递)。
    RecordFire,
    /// 投递写: 任务执行完成记账 (从待执行清单移除)。
    AckExecution,
}

/// 一次入队的写 (管理写或投递写)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleWrite {
    /// 管理写: 新建调度。
    Create {
        /// 新记录 (id 必须非空且不重复)。
        record: ScheduleRecord,
    },
    /// 管理写: 全记录 compare-and-set 更新; 期望不符返回 [`SCHEDULE_CONFLICT_CODE`]。
    Replace {
        /// 期望的完整存储记录。
        expected: ScheduleRecord,
        /// 替换后的完整记录 (版本号由存储收敛为期望版本 + 1)。
        next: ScheduleRecord,
    },
    /// 管理写: 删除调度。
    Delete {
        /// 被删除的调度 id。
        id: String,
    },
    /// 管理写: 暂停/恢复。
    SetStatus {
        /// 目标调度 id。
        id: String,
        /// 目标状态。
        status: ScheduleStatus,
        /// 写入时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 投递写: 发生时点入队 (补发纪律的唯一入口)。
    RecordFire {
        /// 调度 id。
        id: String,
        /// 发生时点 (epoch 毫秒)。
        occurrence_ms: i64,
        /// 记账时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 投递写: 任务执行完成记账 (诚实声明中的第二次持久写)。
    AckExecution {
        /// 调度 id。
        schedule_id: String,
        /// 发生时点 (epoch 毫秒)。
        occurrence_ms: i64,
        /// 记账时刻 (epoch 毫秒)。
        at_ms: i64,
    },
}

impl ScheduleWrite {
    /// 写的种类 (FIFO 队列的观测标签)。
    pub const fn kind(&self) -> ScheduleWriteKind {
        match self {
            Self::Create { .. } => ScheduleWriteKind::Create,
            Self::Replace { .. } => ScheduleWriteKind::Replace,
            Self::Delete { .. } => ScheduleWriteKind::Delete,
            Self::SetStatus { .. } => ScheduleWriteKind::SetStatus,
            Self::RecordFire { .. } => ScheduleWriteKind::RecordFire,
            Self::AckExecution { .. } => ScheduleWriteKind::AckExecution,
        }
    }
}

/// 一次成功应用的写回执 (FIFO 派号 + 种类)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReceipt {
    /// FIFO 派号 (按入队序单调递增)。
    pub seq: u64,
    /// 写的种类。
    pub kind: ScheduleWriteKind,
}

/// 单条写的结局: 冲突/不存在等前置失败逐条回执, 不吞掉后续写。
pub type WriteOutcome = Result<WriteReceipt, ScheduleError>;

/// 持久调度档 (内存镜像 + 单一 FIFO 写队列)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ScheduleState {
    /// 已应用的最大 FIFO 派号 (跨重启单调)。
    write_seq: u64,
    /// 全部记录 (按 id 定序, 应用序确定)。
    records: BTreeMap<String, ScheduleRecord>,
    /// 待执行投递清单 (入队后、执行完成记账前)。
    pending: Vec<ScheduleDelivery>,
}

/// 持久调度库: 触发计算 + 补发纪律 + 全记录 compare-and-set + 单一 FIFO 写队列。
pub struct ScheduleStore {
    path: PathBuf,
    state: ScheduleState,
    /// 下一张 FIFO 派号。
    ticket: u64,
    /// 待应用写 (管理写与投递写共一队列)。
    queue: VecDeque<(u64, ScheduleWrite)>,
    /// 已成功应用的写回执 (观测口, 不落盘)。
    applied: Vec<WriteReceipt>,
}

impl ScheduleStore {
    /// 打开 (或初始化) 指定路径的调度档: 文件不存在即空库。
    pub fn open(path: impl Into<PathBuf>) -> ScheduleResult<Self> {
        let path = path.into();
        let state = load_state(&path)?;
        let ticket = state.write_seq.saturating_add(1);
        Ok(Self {
            path,
            state,
            ticket,
            queue: VecDeque::new(),
            applied: Vec::new(),
        })
    }

    /// 持久档路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 已应用的最大 FIFO 派号。
    pub fn write_seq(&self) -> u64 {
        self.state.write_seq
    }

    /// 待应用写条数 (队列非空 = 有写未落盘)。
    pub fn pending_writes(&self) -> usize {
        self.queue.len()
    }

    /// 已成功应用的写回执 (FIFO 应用序)。
    pub fn applied_writes(&self) -> &[WriteReceipt] {
        &self.applied
    }

    /// 按 id 取记录。
    pub fn get(&self, id: &str) -> Option<&ScheduleRecord> {
        self.state.records.get(id)
    }

    /// 全部记录 (按 id 定序)。
    pub fn records(&self) -> impl Iterator<Item = &ScheduleRecord> {
        self.state.records.values()
    }

    /// 待执行投递清单: 入队后、执行完成记账前的投递 (重启后仍可再次投递)。
    pub fn unacknowledged_deliveries(&self) -> &[ScheduleDelivery] {
        &self.state.pending
    }

    /// 管理写/投递写入队: 按入队序派 FIFO 号, 不立即落盘。
    pub fn enqueue(&mut self, write: ScheduleWrite) -> u64 {
        let ticket = self.ticket;
        self.ticket = self.ticket.saturating_add(1);
        self.queue.push_back((ticket, write));
        ticket
    }

    /// 按 FIFO 应用队列中的全部写: 每写一次原子落盘, 前置失败逐条回执并丢弃该写。
    pub fn drain(&mut self) -> Vec<WriteOutcome> {
        let mut outcomes = Vec::with_capacity(self.queue.len());
        while let Some((ticket, write)) = self.queue.pop_front() {
            match self.apply_one(ticket, write) {
                Ok(receipt) => {
                    self.applied.push(receipt.clone());
                    outcomes.push(Ok(receipt));
                }
                Err(err) => outcomes.push(Err(err)),
            }
        }
        outcomes
    }

    /// 单写便捷口: 入队一条并立刻按 FIFO 应用。
    pub fn apply(&mut self, write: ScheduleWrite) -> ScheduleResult<WriteReceipt> {
        self.enqueue(write);
        match self.drain().into_iter().next() {
            Some(outcome) => outcome,
            None => Err(ScheduleError::InvalidInput {
                reason: "queued write vanished before apply".to_string(),
            }),
        }
    }

    /// 管理写: 新建调度。
    pub fn create(&mut self, record: ScheduleRecord) -> ScheduleResult<WriteReceipt> {
        self.apply(ScheduleWrite::Create { record })
    }

    /// 管理写: 全记录 compare-and-set 更新; 不一致返回
    /// [`ScheduleError::Conflict`] (code [`SCHEDULE_CONFLICT_CODE`]), 不覆盖。
    pub fn compare_and_set(
        &mut self,
        expected: ScheduleRecord,
        next: ScheduleRecord,
    ) -> ScheduleResult<WriteReceipt> {
        self.apply(ScheduleWrite::Replace { expected, next })
    }

    /// 管理写: 删除调度。
    pub fn delete(&mut self, id: impl Into<String>) -> ScheduleResult<WriteReceipt> {
        self.apply(ScheduleWrite::Delete { id: id.into() })
    }

    /// 管理写: 设置状态 (暂停/恢复)。
    pub fn set_status(
        &mut self,
        id: impl Into<String>,
        status: ScheduleStatus,
        at_ms: i64,
    ) -> ScheduleResult<WriteReceipt> {
        self.apply(ScheduleWrite::SetStatus {
            id: id.into(),
            status,
            at_ms,
        })
    }

    /// 投递写: 任务执行完成记账 (诚实声明中的第二次持久写)。
    pub fn acknowledge_execution(
        &mut self,
        schedule_id: impl Into<String>,
        occurrence_ms: i64,
        at_ms: i64,
    ) -> ScheduleResult<WriteReceipt> {
        self.apply(ScheduleWrite::AckExecution {
            schedule_id: schedule_id.into(),
            occurrence_ms,
            at_ms,
        })
    }

    /// 补发纪律入口: 每个活跃调度至多投递一条 —— 只补不晚于 `now_ms` 的
    /// **最近一次**发生时点, 更早的错过一律丢弃 (重启后首趟同样如此)。
    pub fn collect_due(&mut self, now_ms: i64) -> ScheduleResult<Vec<ScheduleDelivery>> {
        let mut writes = Vec::new();
        let mut deliveries = Vec::new();
        for (id, record) in &self.state.records {
            if record.status != ScheduleStatus::Active {
                continue;
            }
            let Some(occurrence) = record.trigger.latest_occurrence_at_or_before(now_ms) else {
                continue;
            };
            if record
                .last_fired_at_ms
                .is_some_and(|last| occurrence <= last)
            {
                continue;
            }
            writes.push(ScheduleWrite::RecordFire {
                id: id.clone(),
                occurrence_ms: occurrence,
                at_ms: now_ms,
            });
            deliveries.push(ScheduleDelivery {
                schedule_id: id.clone(),
                occurrence_ms: occurrence,
                fired_at_ms: now_ms,
                payload: record.payload.clone(),
            });
        }
        for write in writes {
            self.enqueue(write);
        }
        for outcome in self.drain() {
            outcome?;
        }
        Ok(deliveries)
    }

    /// 应用一条写: 纯状态迁移 + 原子落盘 + 提交 (落盘失败不提交, 状态不漂移)。
    fn apply_one(&mut self, ticket: u64, write: ScheduleWrite) -> ScheduleResult<WriteReceipt> {
        let kind = write.kind();
        let mut next = self.state.clone();
        transition(&mut next, write, ticket)?;
        self.persist(&next)?;
        self.state = next;
        Ok(WriteReceipt { seq: ticket, kind })
    }

    /// 原子落盘 (持久档 + 文件锁): 只消费 `storage_atomic` 的两档原子写。
    fn persist(&self, state: &ScheduleState) -> ScheduleResult<()> {
        let bytes = serde_json::to_vec(state).map_err(|e| ScheduleError::Corrupt {
            reason: format!("serialize schedule state: {e}"),
        })?;
        let lock_path = storage_atomic::lock_path_for(&self.path);
        let written = storage_atomic::with_file_lock(&lock_path, || {
            storage_atomic::write_atomic_durable(&self.path, &bytes, DEFAULT_FILE_MODE)
        })
        .map_err(|e| ScheduleError::Io {
            operation: "lock schedule store",
            reason: e.to_string(),
        })?;
        written.map_err(|e| ScheduleError::Io {
            operation: "write schedule store",
            reason: e.to_string(),
        })
    }
}

/// 状态迁移 (纯函数): 前置检查 + 变更, 不做 IO。
fn transition(state: &mut ScheduleState, write: ScheduleWrite, ticket: u64) -> ScheduleResult<()> {
    match write {
        ScheduleWrite::Create { mut record } => {
            if record.id.is_empty() {
                return Err(ScheduleError::InvalidInput {
                    reason: "schedule id must not be empty".to_string(),
                });
            }
            record.trigger.validate()?;
            if state.records.contains_key(&record.id) {
                return Err(ScheduleError::DuplicateId { id: record.id });
            }
            record.revision = 1;
            state.records.insert(record.id.clone(), record);
        }
        ScheduleWrite::Replace { expected, mut next } => {
            let Some(stored) = state.records.get(&expected.id) else {
                return Err(ScheduleError::NotFound { id: expected.id });
            };
            // 全记录 compare-and-set: 逐字段相等才替换, 冲突一律不覆盖。
            if *stored != expected {
                return Err(ScheduleError::Conflict {
                    id: expected.id,
                    expected_revision: expected.revision,
                });
            }
            if next.id != expected.id {
                return Err(ScheduleError::InvalidInput {
                    reason: "next record must keep the same schedule id".to_string(),
                });
            }
            next.trigger.validate()?;
            next.revision = stored.revision.saturating_add(1);
            let id = next.id.clone();
            state.records.insert(id, next);
        }
        ScheduleWrite::Delete { id } => {
            if state.records.remove(&id).is_none() {
                return Err(ScheduleError::NotFound { id });
            }
        }
        ScheduleWrite::SetStatus { id, status, at_ms } => {
            let Some(record) = state.records.get_mut(&id) else {
                return Err(ScheduleError::NotFound { id });
            };
            record.status = status;
            record.updated_at_ms = at_ms;
            record.revision = record.revision.saturating_add(1);
        }
        ScheduleWrite::RecordFire {
            id,
            occurrence_ms,
            at_ms,
        } => {
            let Some(record) = state.records.get_mut(&id) else {
                return Err(ScheduleError::NotFound { id });
            };
            if record
                .last_fired_at_ms
                .is_some_and(|last| occurrence_ms <= last)
            {
                return Err(ScheduleError::InvalidInput {
                    reason: format!(
                        "occurrence {occurrence_ms} already recorded for schedule {id}"
                    ),
                });
            }
            record.last_fired_at_ms = Some(occurrence_ms);
            record.fire_count = record.fire_count.saturating_add(1);
            record.updated_at_ms = at_ms;
            record.revision = record.revision.saturating_add(1);
            if matches!(record.trigger, ScheduleTrigger::After { .. }) {
                record.status = ScheduleStatus::Completed;
            }
            let payload = record.payload.clone();
            state.pending.push(ScheduleDelivery {
                schedule_id: id,
                occurrence_ms,
                fired_at_ms: at_ms,
                payload,
            });
        }
        ScheduleWrite::AckExecution {
            schedule_id,
            occurrence_ms,
            at_ms: _,
        } => {
            let Some(pos) = state
                .pending
                .iter()
                .position(|d| d.schedule_id == schedule_id && d.occurrence_ms == occurrence_ms)
            else {
                return Err(ScheduleError::NotFound { id: schedule_id });
            };
            state.pending.remove(pos);
        }
    }
    state.write_seq = ticket;
    Ok(())
}

/// 读档: 缺失即空库; 内容无法解析 fail-closed 报 [`ScheduleError::Corrupt`]。
fn load_state(path: &Path) -> ScheduleResult<ScheduleState> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ScheduleState::default());
        }
        Err(err) => {
            return Err(ScheduleError::Io {
                operation: "read schedule store",
                reason: err.to_string(),
            });
        }
    };
    serde_json::from_slice(&bytes).map_err(|e| ScheduleError::Corrupt {
        reason: format!("parse schedule state: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定测试锚点: 任意整数毫秒, 全部用例显式传时刻, 不读挂钟。
    const T0: i64 = 1_700_000_000_000;
    const HOUR_MS: i64 = 3_600_000;
    const MINUTE_MS: i64 = 60_000;

    fn every_record(id: &str, every_ms: i64, anchor_ms: i64) -> ScheduleRecord {
        ScheduleRecord {
            id: id.to_string(),
            revision: 0,
            trigger: ScheduleTrigger::Every {
                every_ms,
                anchor_ms,
            },
            status: ScheduleStatus::Active,
            payload: format!("payload-{id}"),
            created_at_ms: anchor_ms,
            updated_at_ms: anchor_ms,
            last_fired_at_ms: None,
            fire_count: 0,
        }
    }

    fn store_at(dir: &tempfile::TempDir) -> ScheduleStore {
        ScheduleStore::open(dir.path().join("schedule.json")).expect("open schedule store")
    }

    #[test]
    fn after_trigger_fires_once_then_completes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(ScheduleRecord {
                trigger: ScheduleTrigger::After {
                    at_ms: T0 + HOUR_MS,
                },
                ..every_record("one-shot", 1, T0)
            })
            .expect("create");

        // 到点前不投; 到点后恰一次; 之后完成, 不再产生发生时点。
        assert!(store.collect_due(T0 + HOUR_MS - 1).expect("due").is_empty());
        let due = store.collect_due(T0 + HOUR_MS).expect("due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].occurrence_ms, T0 + HOUR_MS);
        assert_eq!(
            store.get("one-shot").expect("record").status,
            ScheduleStatus::Completed
        );
        assert!(store
            .collect_due(T0 + 10 * HOUR_MS)
            .expect("due")
            .is_empty());
    }

    #[test]
    fn every_trigger_follows_interval_from_anchor() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(every_record("tick", HOUR_MS, T0))
            .expect("create");

        // 锚点是第一个发生时点; 未到锚点不投。
        assert!(store.collect_due(T0 - 1).expect("due").is_empty());
        let first = store.collect_due(T0).expect("due");
        assert_eq!(first[0].occurrence_ms, T0);
        // 锚点 + 1 间隔 = 第二个发生时点。
        let second = store.collect_due(T0 + HOUR_MS).expect("due");
        assert_eq!(second[0].occurrence_ms, T0 + HOUR_MS);
        // 同一发生时点不重复投递。
        assert!(store.collect_due(T0 + HOUR_MS).expect("due").is_empty());
    }

    #[test]
    fn cron_subset_fixed_times_fire_daily_and_weekly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(ScheduleRecord {
                trigger: ScheduleTrigger::Cron {
                    spec: CronSubset::Daily {
                        hour: 8,
                        minute: 30,
                        utc_offset_minutes: 0,
                    },
                },
                ..every_record("daily", 1, T0)
            })
            .expect("create daily");
        // 目标星期取周一 (1=周一, 0=周日)。
        store
            .create(ScheduleRecord {
                trigger: ScheduleTrigger::Cron {
                    spec: CronSubset::Weekly {
                        weekday: 1,
                        hour: 9,
                        minute: 0,
                        utc_offset_minutes: 0,
                    },
                },
                ..every_record("weekly", 1, T0)
            })
            .expect("create weekly");

        // 日序 10 是周日, 当天 10:00 巡检: 每日型补当天 08:30,
        // 每周型补最近的周一 (日序 4) 09:00。
        let day = 10;
        let now = day * DAY_MS + 10 * HOUR_MS;
        let due = store.collect_due(now).expect("due");
        let daily = due
            .iter()
            .find(|d| d.schedule_id == "daily")
            .expect("daily");
        assert_eq!(
            daily.occurrence_ms,
            day * DAY_MS + 8 * HOUR_MS + 30 * MINUTE_MS
        );
        let weekly = due
            .iter()
            .find(|d| d.schedule_id == "weekly")
            .expect("weekly");
        assert_eq!(weekly.occurrence_ms, 4 * DAY_MS + 9 * HOUR_MS);

        // 次日 07:00 巡检: 每日型未到 08:30 不投 (补发游标已过)。
        assert!(store
            .collect_due((day + 1) * DAY_MS + 7 * HOUR_MS)
            .expect("due")
            .is_empty());
    }

    #[test]
    fn restart_catches_up_only_the_latest_missed_occurrence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(every_record("tick", HOUR_MS, T0))
            .expect("create");
        let first = store.collect_due(T0).expect("due");
        store
            .acknowledge_execution("tick", first[0].occurrence_ms, T0)
            .expect("ack");
        drop(store); // 停机: 期间错过 T0+1h .. T0+10h 共 10 个发生时点。

        let mut reopened = store_at(&dir);
        let now = T0 + 10 * HOUR_MS;
        let due = reopened.collect_due(now).expect("due");
        // 补发纪律: 只补最近一次错过的发生时点, 不风暴。
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].occurrence_ms, now);
        assert_eq!(
            reopened.unacknowledged_deliveries().len(),
            1,
            "待执行清单也只有一条, 不积压 10 条"
        );
    }

    #[test]
    fn cas_update_conflicts_return_schedule_conflict_without_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(every_record("cas", HOUR_MS, T0))
            .expect("create");
        let stored = store.get("cas").expect("record").clone();
        assert_eq!(stored.revision, 1);

        // 陈旧期望 (payload 是旧值) 与存储记录不一致 → schedule_conflict, 不覆盖。
        let mut stale = stored.clone();
        stale.payload = "stale-payload".to_string();
        let mut next = stored.clone();
        next.payload = "next-payload".to_string();
        let err = store
            .compare_and_set(stale, next.clone())
            .expect_err("stale expected must conflict");
        assert_eq!(err.code().as_str(), SCHEDULE_CONFLICT_CODE);
        assert_eq!(err.code(), ScheduleErrorCode::Conflict);
        assert_eq!(
            store.get("cas").expect("record"),
            &stored,
            "冲突不得覆盖存储记录"
        );

        // 期望逐字段相等 → 替换成功, 版本号 +1。
        let applied = store
            .compare_and_set(stored.clone(), next)
            .expect("matching expected applies");
        assert_eq!(applied.kind, ScheduleWriteKind::Replace);
        let after = store.get("cas").expect("record");
        assert_eq!(after.payload, "next-payload");
        assert_eq!(after.revision, 2);
    }

    #[test]
    fn admin_and_delivery_writes_share_one_fifo_queue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        // 管理写与投递写交错入队, 共用同一 FIFO; 未 drain 前不落盘。
        let t1 = store.enqueue(ScheduleWrite::Create {
            record: every_record("fifo", HOUR_MS, T0),
        });
        let t2 = store.enqueue(ScheduleWrite::RecordFire {
            id: "fifo".to_string(),
            occurrence_ms: T0,
            at_ms: T0,
        });
        let t3 = store.enqueue(ScheduleWrite::SetStatus {
            id: "fifo".to_string(),
            status: ScheduleStatus::Paused,
            at_ms: T0 + 1,
        });
        let t4 = store.enqueue(ScheduleWrite::Delete {
            id: "fifo".to_string(),
        });
        assert_eq!((t1, t2, t3, t4), (1, 2, 3, 4), "派号按入队序单调");
        assert_eq!(store.pending_writes(), 4, "入队不等于应用");
        assert!(store.get("fifo").is_none(), "未 drain 前不得落盘生效");

        let outcomes = store.drain();
        assert_eq!(outcomes.len(), 4);
        assert!(outcomes.iter().all(|o| o.is_ok()));
        // 应用序 = 入队序, 不分管理写/投递写。
        let kinds: Vec<ScheduleWriteKind> = store.applied_writes().iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ScheduleWriteKind::Create,
                ScheduleWriteKind::RecordFire,
                ScheduleWriteKind::SetStatus,
                ScheduleWriteKind::Delete,
            ]
        );
        let seqs: Vec<u64> = store.applied_writes().iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3, 4], "FIFO 应用序严格按派号");
    }

    #[test]
    fn crash_between_enqueue_and_execution_redelivers_honestly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(every_record("work", HOUR_MS, T0))
            .expect("create");
        // 第一次持久写: 发生时点入队 (collect_due 的投递写)。
        let due = store.collect_due(T0).expect("due");
        assert_eq!(due.len(), 1);
        // 执行方拿到投递后尚未 acknowledge_execution (第二次持久写) 就崩溃。
        drop(store);

        let mut reopened = store_at(&dir);
        // 诚实声明: 两次持久写不原子, 待执行清单仍持有该发生时点 → 可重复投递。
        let unacked = reopened.unacknowledged_deliveries();
        assert_eq!(unacked.len(), 1);
        assert_eq!(unacked[0].occurrence_ms, T0);
        assert_eq!(unacked[0].schedule_id, "work");
        // 再次投递同一发生时点 (执行方按 (schedule_id, occurrence_ms) 幂等去重)。
        assert_eq!(unacked[0], due[0]);

        // 执行完成记账后移出待执行清单; 重复记账显式失败, 不静默吞。
        reopened
            .acknowledge_execution("work", T0, T0 + 1)
            .expect("ack");
        assert!(reopened.unacknowledged_deliveries().is_empty());
        let err = reopened
            .acknowledge_execution("work", T0, T0 + 2)
            .expect_err("double ack must fail");
        assert_eq!(err.code(), ScheduleErrorCode::NotFound);
    }

    #[test]
    fn delete_and_pause_gate_delivery_until_resumed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = store_at(&dir);
        store
            .create(every_record("pausable", HOUR_MS, T0))
            .expect("create");
        store
            .create(every_record("removable", HOUR_MS, T0))
            .expect("create");
        store
            .collect_due(T0)
            .expect("due")
            .into_iter()
            .for_each(|d| {
                store
                    .acknowledge_execution(d.schedule_id, d.occurrence_ms, T0)
                    .expect("ack");
            });

        // 暂停 + 删除 是管理写, 走同一 FIFO。
        store
            .set_status("pausable", ScheduleStatus::Paused, T0 + 1)
            .expect("pause");
        store.delete("removable").expect("delete");

        // 暂停期间不投; 删除后不投。
        assert!(store.collect_due(T0 + 2 * HOUR_MS).expect("due").is_empty());
        assert!(store.get("removable").is_none());

        // 恢复后按补发纪律至多补最近一次 (不风暴)。
        store
            .set_status("pausable", ScheduleStatus::Active, T0 + 3 * HOUR_MS)
            .expect("resume");
        let due = store.collect_due(T0 + 3 * HOUR_MS).expect("due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].occurrence_ms, T0 + 3 * HOUR_MS);
    }

    #[test]
    fn trigger_validation_rejects_out_of_range_values() {
        assert_eq!(
            ScheduleTrigger::Every {
                every_ms: 0,
                anchor_ms: T0
            }
            .validate()
            .expect_err("zero interval")
            .code(),
            ScheduleErrorCode::InvalidTrigger
        );
        assert_eq!(
            ScheduleTrigger::Cron {
                spec: CronSubset::Daily {
                    hour: 24,
                    minute: 0,
                    utc_offset_minutes: 0
                }
            }
            .validate()
            .expect_err("hour out of range")
            .code(),
            ScheduleErrorCode::InvalidTrigger
        );
        assert_eq!(
            ScheduleTrigger::Cron {
                spec: CronSubset::Weekly {
                    weekday: 7,
                    hour: 0,
                    minute: 0,
                    utc_offset_minutes: 0
                }
            }
            .validate()
            .expect_err("weekday out of range")
            .code(),
            ScheduleErrorCode::InvalidTrigger
        );
    }

    #[test]
    fn store_round_trips_through_the_durable_tier() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let mut store = store_at(&dir);
            store
                .create(every_record("persisted", HOUR_MS, T0))
                .expect("create");
            store.collect_due(T0).expect("due");
        }
        let reopened = store_at(&dir);
        let record = reopened.get("persisted").expect("restored");
        assert_eq!(record.fire_count, 1);
        assert_eq!(record.last_fired_at_ms, Some(T0));
        assert_eq!(reopened.write_seq(), 2);
    }
}
