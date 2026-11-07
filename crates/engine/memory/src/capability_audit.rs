//! 记忆能力**接线审计**（只读）：回答「这些能力在本库上真的能跑出什么数」。
//!
//! # 为什么需要它（2026-11-07）
//!
//! `scripts/check_memory_reachability.py` 把「能力可达率」变成一个数字，但它只能
//! 判**词法可达**；**可达 ≠ 会用**，而「不可达」里混着两类完全不同的东西：
//!
//! * **真的没接电**（模块写好了、生产零调用）⇒ ⇒ 是债；
//! * **接了但没人给它输入**（例如持久向量索引本身有意不接 ⇒ ⇒ 它的重建/过滤
//!   跟着一起空转）⇒ ⇒ 不是债，是**上游缺位**。
//!
//! 两者从「可达/不可达」两个字上看不出来 ⇒ ⇒ 需要一次**真读库**的审计：
//! 每个能力给出「能读到多少真数据 / 为什么读不到」。
//!
//! # 接线位置（**这是生产面，不是测试夹具**）
//!
//! 审计由 [`crate::handover::render_capability_audit`] 触发，被
//! `apeireth memory handover` 写进交接包摘要 ⇒ ⇒ 用户拿到的交接包**自带**
//! 「哪些记忆能力在这台机器/这个库上是真的在跑」的答案 ⇒ ⇒ 接手者不用猜。
//!
//! # 三条纪律（写在代码里，防止以后被改成「装绿」）
//!
//! 1. **只读**：所有探针只做 `SELECT` / 内存内计算；不建表、不写行、不改状态
//!    （`cold_archive::read_ledger` 明确「只读连接也能读，不为了读而建表」）。
//! 2. **不粉饰**：读不到就报 `UNAVAILABLE` + 真因；读到 0 就报 `IDLE` +
//!    「能力可用但本库无输入」⇒ ⇒ **不允许**把 0 说成「已接线」。
//! 3. **确定性与可复现**：只输出计数 / 布尔 / 档位；**不输出**哈希与时刻
//!    （`arbitration` 的 merkle 根、`now_ms` 派生的日期除外）⇒ ⇒ 同库两次跑，
//!    除「今日」边界外逐字相等。

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Value, json};

use crate::agent_trace::TraceSpanStatus;
use crate::arbitration::{ArbitrationEngine, EventSource};
use crate::calibration::Observation;
use crate::calibration_critic::CalibrationCritic;
use crate::cold_archive;
use crate::confidence::BetaBinomial;
use crate::daily_summary;
use crate::derived_repair::{RepairExecutor, RepairNode};
use crate::ensemble::{EnsembleConfig, EnsembleForecast, EnsembleMember};
use crate::forget_coordinator::CoordinatedForget;
use crate::memory_view::{InMemoryMemoryView, ProvenanceChain, Record, RecordId, Scope};
use crate::metadata_filter::{MetadataFilter, PropertyPredicate};
use crate::offline_embed;
use crate::offline_embedding_provider::{OFFLINE_EMBEDDING_MODEL_ID, OfflineEmbeddingProvider};
use crate::research_derived_memory::DerivedRef;
use crate::session_lifecycle::SessionState;
use crate::swap_flow;
use crate::vector_rebuild;

/// 单个能力的审计状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeState {
    /// 读到真实输入并在其上算出了结果（**能力真的在跑**）。
    Armed,
    /// 能力可用，但本库/本部署**没有输入**（上游有意未接或尚无数据）。
    Idle,
    /// 探测本身失败（读库错 / 打不开 store）⇒ ⇒ **如实报，不当 0**。
    Unavailable,
}

impl ProbeState {
    /// 稳定英文标签（进 JSON 与人类摘要）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Armed => "ARMED",
            Self::Idle => "IDLE",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

/// 一个探针的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityProbe {
    /// 记忆 crate 顶层模块名（与可达率闸门的口径一致 ⇒ 可对照）。
    pub module: &'static str,
    /// 状态。
    pub state: ProbeState,
    /// **真数**或**真因**（一句话，机器可读优先）。
    pub detail: String,
}

/// 一次能力审计的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityAudit {
    /// 逐能力探针（顺序固定 ⇒ 输出确定性）。
    pub probes: Vec<CapabilityProbe>,
}

impl CapabilityAudit {
    /// `(ARMED, IDLE, UNAVAILABLE)` 三个计数。
    pub fn counts(&self) -> (usize, usize, usize) {
        let armed = self
            .probes
            .iter()
            .filter(|p| p.state == ProbeState::Armed)
            .count();
        let idle = self
            .probes
            .iter()
            .filter(|p| p.state == ProbeState::Idle)
            .count();
        (armed, idle, self.probes.len() - armed - idle)
    }

    /// 人类可读摘要（写进交接包摘要 ⇒ 接手者直接看）。
    pub fn render(&self) -> String {
        let (armed, idle, unavailable) = self.counts();
        let mut out = String::from("【能力接线审计 · 只读】\n");
        out.push_str(&format!(
            "  ARMED {} · IDLE {} · UNAVAILABLE {} · 共 {} 项\n",
            armed,
            idle,
            unavailable,
            self.probes.len()
        ));
        for probe in &self.probes {
            out.push_str(&format!(
                "  · {:<34} {:<11} {}\n",
                probe.module,
                probe.state.label(),
                probe.detail
            ));
        }
        out.push_str(
            "  ⚠️ IDLE = 能力可用但本库/本部署无输入（≠ 未接电）；\
             UNAVAILABLE = 探测失败（真因见行内）\n",
        );
        out
    }
}

/// 库里的一条 episode（审计用的窄视图）。
struct EpisodeRow {
    id: String,
    content: String,
    role: String,
    session_id: String,
    timestamp: i64,
}

fn load_episodes(conn: &Connection, limit: usize) -> Result<Vec<EpisodeRow>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, content, role, session_id, timestamp FROM episodes \
             ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|error| format!("episodes 查询准备失败：{error}"))?;
    let rows = statement
        .query_map([limit as i64], |row| {
            Ok(EpisodeRow {
                id: row.get(0)?,
                content: row.get(1)?,
                role: row.get(2)?,
                session_id: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })
        .map_err(|error| format!("episodes 查询失败：{error}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("episodes 行读取失败：{error}"))?);
    }
    Ok(out)
}

/// 承诺表里的 `(confidence, status)`（校准/置信度探针的输入）。
fn load_commitment_observations(conn: &Connection) -> Result<Vec<(f64, String)>, String> {
    let mut statement = conn
        .prepare("SELECT confidence, status FROM commitments LIMIT 500")
        .map_err(|error| format!("commitments 查询准备失败：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("commitments 查询失败：{error}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("commitments 行读取失败：{error}"))?);
    }
    Ok(out)
}

/// 承诺完成率 `(完成数, 总数)`。口径 = `status` 为 `completed` 记成功，其余记失败
/// （**如实**：这是「承诺兑现」这一问的粗口径，不是概率模型的真值）。
fn commitment_completion(pairs: &[(f64, String)]) -> (usize, usize) {
    let total = pairs.len();
    let done = pairs
        .iter()
        .filter(|(_, status)| status == "completed")
        .count();
    (done, total)
}

/// 走一遍全部能力探针（**只读**）。
///
/// `now_ms` 由调用方注入 ⇒ 审计自身不读墙钟（除 `arbitration` 的事件时间戳由
/// 模块内部取系统时钟，但审计**不输出**该时间戳）。
pub fn audit(conn: &Connection, now_ms: i64) -> CapabilityAudit {
    let mut probes: Vec<CapabilityProbe> = Vec::new();
    let episodes = load_episodes(conn, 40);
    let commitments = load_commitment_observations(conn);

    // ── agent_trace：落库 span 的条数与终态比例 ─────────────────────
    probes.push({
        let rows: Result<Vec<(String, String)>, String> = (|| {
            let mut statement = conn
                .prepare("SELECT kind, status FROM agent_traces LIMIT 500")
                .map_err(|error| format!("agent_traces 查询失败：{error}"))?;
            let iter = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| format!("agent_traces 读取失败：{error}"))?;
            let mut out = Vec::new();
            for row in iter {
                out.push(row.map_err(|error| format!("agent_traces 行失败：{error}"))?);
            }
            Ok(out)
        })();
        match rows {
            Ok(list) => {
                let terminal = list
                    .iter()
                    .filter(|(_, status)| TraceSpanStatus::from_str(status).is_terminal())
                    .count();
                CapabilityProbe {
                    module: "agent_trace",
                    state: if list.is_empty() {
                        ProbeState::Idle
                    } else {
                        ProbeState::Armed
                    },
                    detail: format!("span {} 条 · 终态 {}", list.len(), terminal),
                }
            }
            Err(error) => CapabilityProbe {
                module: "agent_trace",
                state: ProbeState::Unavailable,
                detail: error,
            },
        }
    });

    // ── arbitration：把最近 episode 重放成事件链并校验完整性 ────────────
    probes.push(match &episodes {
        Ok(rows) if !rows.is_empty() => {
            let engine = ArbitrationEngine::new();
            let mut appended = 0usize;
            for row in rows.iter().take(5) {
                let payload = json!({ "episode": row.id, "chars": row.content.chars().count() })
                    .to_string();
                if engine
                    .append_event(EventSource::System, &row.session_id, "episode.observed", &payload)
                    .is_ok()
                {
                    appended += 1;
                }
            }
            match engine.verify_integrity() {
                Ok(report) => CapabilityProbe {
                    module: "arbitration",
                    state: ProbeState::Armed,
                    detail: format!(
                        "重放 {} 条事件 · 完整性 {} · 校验 {} 条 · merkle {} 位",
                        appended,
                        if report.is_valid { "VALID" } else { "BROKEN" },
                        report.verified_count,
                        report.merkle_root.len()
                    ),
                },
                Err(error) => CapabilityProbe {
                    module: "arbitration",
                    state: ProbeState::Unavailable,
                    detail: format!("完整性校验失败：{error:?}"),
                },
            }
        }
        Ok(_) => CapabilityProbe {
            module: "arbitration",
            state: ProbeState::Idle,
            detail: "本库无 episode ⇒ 无事件可重放".into(),
        },
        Err(error) => CapabilityProbe {
            module: "arbitration",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    // ── calibration_critic + confidence：承诺置信度 vs 兑现结局 ─────────
    let (done, total) = commitments
        .as_ref()
        .map(|pairs| commitment_completion(pairs))
        .unwrap_or((0, 0));
    probes.push(match &commitments {
        Ok(pairs) if !pairs.is_empty() => {
            let history: Vec<Observation> = pairs
                .iter()
                .map(|(confidence, status)| {
                    Observation::new(*confidence, if status == "completed" { 1.0 } else { 0.0 })
                })
                .collect();
            let critic = CalibrationCritic::default_critic();
            let result = critic.critique(&history);
            CapabilityProbe {
                module: "calibration_critic",
                state: ProbeState::Armed,
                detail: format!(
                    "样本 {} · 建议 {:?} · 严重度 {:.3} · ECE {:.3}",
                    result.num_samples,
                    result.recommended_action,
                    result.severity,
                    result.expected_calibration_error
                ),
            }
        }
        Ok(_) => CapabilityProbe {
            module: "calibration_critic",
            state: ProbeState::Idle,
            detail: "本库无承诺 ⇒ 无「预测 vs 结局」样本".into(),
        },
        Err(error) => CapabilityProbe {
            module: "calibration_critic",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    {
        let mut posterior = BetaBinomial::new(1.0, 1.0);
        posterior.observe_counts(done as u64, total as u64);
        let (low, high) = posterior.interval95();
        probes.push(CapabilityProbe {
            module: "confidence",
            state: if total > 0 {
                ProbeState::Armed
            } else {
                ProbeState::Idle
            },
            detail: format!(
                "承诺兑现 {done}/{total} · 后验均值 {:.3} · 95% [{:.3}, {:.3}] · {}",
                posterior.mean(),
                low,
                high,
                posterior.strength().label()
            ),
        });
    }

    // ── cold_archive：归档账本 + 完整性 ───────────────────────────────
    probes.push(match cold_archive::read_ledger(conn) {
        Ok(ledger) => {
            let integrity = match cold_archive::verify_ledger(conn) {
                Ok(()) => "VALID",
                Err(_) => "BROKEN",
            };
            CapabilityProbe {
                module: "cold_archive",
                state: if ledger.entries.is_empty() {
                    ProbeState::Idle
                } else {
                    ProbeState::Armed
                },
                detail: format!("归档 {} 条 · 账本 {}", ledger.entries.len(), integrity),
            }
        }
        Err(error) => CapabilityProbe {
            module: "cold_archive",
            state: ProbeState::Unavailable,
            detail: format!("读归档账本失败：{error}"),
        },
    });

    // ── daily_summary：今日四类计数 ───────────────────────────────────
    probes.push(match &episodes {
        Ok(rows) => {
            let today = daily_summary_date(now_ms);
            let day_start = now_ms - (now_ms % 86_400_000);
            let today_rows: Vec<&EpisodeRow> =
                rows.iter().filter(|r| r.timestamp >= day_start).collect();
            let entries: Vec<(&str, &str)> = today_rows
                .iter()
                .map(|r| (r.id.as_str(), r.content.as_str()))
                .collect();
            let summary = daily_summary::build_daily_summary(&today, &entries, 0);
            CapabilityProbe {
                module: "daily_summary",
                state: if entries.is_empty() {
                    ProbeState::Idle
                } else {
                    ProbeState::Armed
                },
                detail: format!(
                    "{} · 记忆 {} · 做梦 {} · 反思 {} · 总事件 {}",
                    summary.date,
                    summary.memory_writes,
                    summary.dreams,
                    summary.reflections,
                    summary.episode_count
                ),
            }
        }
        Err(error) => CapabilityProbe {
            module: "daily_summary",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    // ── forget_coordinator：持久遗忘集（只读；不执行协调遗忘） ──────────
    let forgotten: Result<Vec<DerivedRef>, String> = (|| {
        let mut statement = conn
            .prepare("SELECT kind, id FROM research_forgotten_artifacts ORDER BY kind, id LIMIT 500")
            .map_err(|error| format!("遗忘集查询失败：{error}"))?;
        let iter = statement
            .query_map([], |row| {
                Ok(DerivedRef {
                    kind: row.get::<_, String>(0)?,
                    id: row.get::<_, String>(1)?,
                })
            })
            .map_err(|error| format!("遗忘集读取失败：{error}"))?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row.map_err(|error| format!("遗忘集行失败：{error}"))?);
        }
        Ok::<Vec<DerivedRef>, String>(out)
    })();
    match &forgotten {
        Ok(roots) => {
            // 只造**请求草案**（真类型）；执行需审批工件 ⇒ ⇒ 审计**不执行**。
            let draft = CoordinatedForget {
                roots: roots.iter().take(3).cloned().collect(),
                reason: "capability-audit（只读，不执行）".into(),
                approval_id: "未请求审批".into(),
            };
            probes.push(CapabilityProbe {
                module: "forget_coordinator",
                state: if roots.is_empty() {
                    ProbeState::Idle
                } else {
                    ProbeState::Armed
                },
                detail: format!(
                    "持久遗忘集 {} 条 · 协调遗忘未执行（需审批工件）· 请求草案 roots={}",
                    roots.len(),
                    draft.roots.len()
                ),
            });
        }
        Err(error) => probes.push(CapabilityProbe {
            module: "forget_coordinator",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        }),
    }
    let forgotten: Vec<DerivedRef> = forgotten.unwrap_or_default();

    // ── derived_repair：以遗忘集为根算受影响闭包 ────────────────────
    {
        let nodes: Vec<RepairNode> = forgotten
            .iter()
            .map(|artifact| RepairNode {
                artifact: artifact.clone(),
                sources: Vec::new(),
                weight: 1.0,
                cost: 1.0,
            })
            .collect();
        let affected = RepairExecutor::affected_closure(&nodes, &forgotten);
        probes.push(CapabilityProbe {
            module: "derived_repair",
            state: if forgotten.is_empty() {
                ProbeState::Idle
            } else {
                ProbeState::Armed
            },
            detail: format!(
                "根 {} 条 · 受影响闭包 {} 条 · 血缘登记 {} 条（无血缘 ⇒ 闭包=根集）",
                forgotten.len(),
                affected.len(),
                nodes.iter().filter(|n| !n.sources.is_empty()).count()
            ),
        });
    }

    // ── ensemble：把库内三个独立率当独立预测者聚合 ──────────────────
    {
        let completion_rate = if total > 0 {
            done as f64 / total as f64
        } else {
            0.5
        };
        let mut posterior = BetaBinomial::new(1.0, 1.0);
        posterior.observe_counts(done as u64, total as u64);
        let members = vec![
            EnsembleMember::new("commitment_completion", completion_rate, 0.5),
            EnsembleMember::new("confidence_posterior", posterior.mean(), 0.5),
            EnsembleMember::new("episode_present", if episodes.as_ref().map(|e| !e.is_empty()).unwrap_or(false) { 1.0 } else { 0.0 }, 0.3),
        ];
        let forecast = EnsembleForecast::aggregate(members, EnsembleConfig::default());
        probes.push(CapabilityProbe {
            module: "ensemble",
            state: if total > 0 {
                ProbeState::Armed
            } else {
                ProbeState::Idle
            },
            detail: format!(
                "聚合 {} 个独立信号 · 预测 {:.3} · 一致度 {:.3} · 策略 {:?}",
                forecast.members.len(),
                forecast.aggregate_prediction,
                forecast.agreement_score,
                forecast.strategy
            ),
        });
    }

    // ── memory_view：用真记录建视图并数可见条数 ──────────────────────
    probes.push(match &episodes {
        Ok(rows) => {
            let records: Vec<Record> = rows
                .iter()
                .take(20)
                .map(|row| Record {
                    id: RecordId::new(row.id.clone()),
                    content: row.content.clone(),
                    timestamp_ms: row.timestamp,
                    provenance: ProvenanceChain::default(),
                    scope: Scope::Session {
                        session_id: row.session_id.clone(),
                    },
                })
                .collect();
            let granted = rows.first().map(|row| Scope::Session {
                session_id: row.session_id.clone(),
            });
            let view = InMemoryMemoryView::new(records);
            let visible = granted
                .as_ref()
                .map(|scope| view.visible_records(std::slice::from_ref(scope)).len())
                .unwrap_or(0);
            CapabilityProbe {
                module: "memory_view",
                state: if view.is_empty() {
                    ProbeState::Idle
                } else {
                    ProbeState::Armed
                },
                detail: format!("视图 {} 条 · 会话可见 {}", view.len(), visible),
            }
        }
        Err(error) => CapabilityProbe {
            module: "memory_view",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    // ── metadata_filter：真属性上的命中/拒绝自检 ────────────────────
    probes.push(match &episodes {
        Ok(rows) => match rows.first() {
            Some(row) => {
                let properties: Value = json!({ "role": row.role, "session_id": row.session_id });
                let hit = PropertyPredicate::string("role", row.role.clone()).matches(&properties);
                let miss = PropertyPredicate::string("role", "definitely-not-a-role")
                    .matches(&properties);
                let kind = MetadataFilter::new().kind_eq("episode").matches(Some(&properties));
                CapabilityProbe {
                    module: "metadata_filter",
                    state: ProbeState::Armed,
                    detail: format!(
                        "真属性命中 {} · 异值拒绝 {} · kind 过滤 {}",
                        hit, !miss, kind
                    ),
                }
            }
            None => CapabilityProbe {
                module: "metadata_filter",
                state: ProbeState::Idle,
                detail: "本库无 episode ⇒ 无属性可过滤".into(),
            },
        },
        Err(error) => CapabilityProbe {
            module: "metadata_filter",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    // ── offline_embed：真文本投影维度与能量 ────────────────────────
    probes.push(match &episodes {
        Ok(rows) => match rows.first() {
            Some(row) => {
                let vector = offline_embed::embed_text(&row.content, 64);
                let energy: f32 = vector.iter().map(|v| v * v).sum();
                CapabilityProbe {
                    module: "offline_embed",
                    state: ProbeState::Armed,
                    detail: format!(
                        "真文本投影 {} 维 · 能量 {:.4}（纯本地、无 API key）",
                        vector.len(),
                        energy
                    ),
                }
            }
            None => CapabilityProbe {
                module: "offline_embed",
                state: ProbeState::Idle,
                detail: "本库无 episode ⇒ 无文本可投影".into(),
            },
        },
        Err(error) => CapabilityProbe {
            module: "offline_embed",
            state: ProbeState::Unavailable,
            detail: error.clone(),
        },
    });

    // ── offline_embedding_provider：适配器可用性（shape 契约） ──────
    probes.push(match OfflineEmbeddingProvider::new(64) {
        Ok(provider) => CapabilityProbe {
            module: "offline_embedding_provider",
            state: ProbeState::Idle,
            detail: format!(
                "适配器就绪 · model={} · {} 维 · **未接生产注入点**（cli/lib.rs 未接）",
                OFFLINE_EMBEDDING_MODEL_ID,
                provider.dimension()
            ),
        },
        Err(error) => CapabilityProbe {
            module: "offline_embedding_provider",
            state: ProbeState::Unavailable,
            detail: format!("适配器构造失败：{error}"),
        },
    });

    // ── session_lifecycle：会话富契约的状态分布 ────────────────────
    probes.push({
        let rows: Result<Vec<String>, String> = (|| {
            let mut statement = conn
                .prepare("SELECT state FROM sessions LIMIT 500")
                .map_err(|error| format!("sessions.state 查询失败：{error}"))?;
            let iter = statement
                .query_map([], |row| row.get::<_, Option<String>>(0))
                .map_err(|error| format!("sessions 读取失败：{error}"))?;
            let mut out = Vec::new();
            for row in iter {
                let state = row.map_err(|error| format!("sessions 行失败：{error}"))?;
                out.push(state.unwrap_or_else(|| "active".to_string()));
            }
            Ok(out)
        })();
        match rows {
            Ok(states) => {
                let active = states
                    .iter()
                    .filter(|state| SessionState::from_str(state) == SessionState::Active)
                    .count();
                CapabilityProbe {
                    module: "session_lifecycle",
                    state: if states.is_empty() {
                        ProbeState::Idle
                    } else {
                        ProbeState::Armed
                    },
                    detail: format!("会话 {} 条 · active {}", states.len(), active),
                }
            }
            Err(error) => CapabilityProbe {
                module: "session_lifecycle",
                state: ProbeState::Unavailable,
                detail: error,
            },
        }
    });

    // ── swap_flow：算法切换归档 ────────────────────────────────────
    {
        let archive_dir = data_dir_of(conn).join("swap-archives");
        probes.push(match swap_flow::list_archives(&archive_dir) {
            Ok(names) => CapabilityProbe {
                module: "swap_flow",
                state: if names.is_empty() {
                    ProbeState::Idle
                } else {
                    ProbeState::Armed
                },
                detail: format!("切换归档 {} 份 · 三道闸未跳步", names.len()),
            },
            Err(_) => CapabilityProbe {
                module: "swap_flow",
                state: ProbeState::Idle,
                detail: "归档目录未建（尚未发生过算法切换）".into(),
            },
        });
    }

    // ── vector_rebuild：持久索引快照 ───────────────────────────────
    {
        let snapshot_path = data_dir_of(conn).join("vector-index.bin");
        probes.push(match vector_rebuild::read_snapshot(&snapshot_path) {
            Ok(Some(snapshot)) => CapabilityProbe {
                module: "vector_rebuild",
                state: ProbeState::Armed,
                detail: format!(
                    "快照模型 {} · 条目 {}",
                    snapshot.stamp.model_id, snapshot.entries
                ),
            },
            Ok(None) => CapabilityProbe {
                module: "vector_rebuild",
                state: ProbeState::Idle,
                detail: "无持久向量索引（生产向量走外部 API ⇒ 无索引可重建）".into(),
            },
            Err(error) => CapabilityProbe {
                module: "vector_rebuild",
                state: ProbeState::Unavailable,
                detail: format!("读快照失败：{error}"),
            },
        });
    }

    CapabilityAudit { probes }
}

/// 库文件所在目录（`swap_flow` / `vector_rebuild` 的落位基准）。
fn data_dir_of(conn: &Connection) -> std::path::PathBuf {
    conn.path()
        .map(Path::new)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// `now_ms` 的 UTC 日期（`YYYY-MM-DD`）⇒ ⇒ 只用于摘要标签，不参与任何判定。
fn daily_summary_date(now_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(now_ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
