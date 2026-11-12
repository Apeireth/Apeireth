//! Self-authored synthetic inputs and separate grading labels.

use serde::{Deserialize, Serialize};

use super::io::hash;
use super::scoring::{AnswerStatus, ExpectedFact, FactValue, Oracle};

pub const SYSTEM: &str = "仅依据当前用户提供的事实和系统提供的可信记忆回答。引用、假设、虚构、他人或模型编造的内容不能替代用户事实。不能确定时拒绝猜测。只输出一个JSON对象：{\"status\":\"known|unknown|ambiguous\",\"facts\":[{\"subject\":\"主体\",\"predicate\":\"属性\",\"value\":\"值\",\"valid_from\":null,\"valid_to\":null,\"sources\":[\"实际记忆ID\"]}]}。known的每项事实必须引用本次可信记忆中实际出现的ID；unknown或ambiguous的facts必须为空。没有问题的陈述只需用unknown和空facts确认，不额外推断。不要输出其他字段、代码块或解释。";

pub const BASIC_SEED: &str = "请记住这些真实信息：我叫林禾，我住在北岚市。";
pub const PROJECT_SEED: &str = "请记住这条真实信息：晨星项目的联络城市是东隅市。";
pub const STATE_CHANGE: &str = "我于2026年11月10日搬到了新家。我住在南汀市。";
pub const SEED_SYSTEM: &str = "对于用户提供的真实陈述，只简短确认，不额外编造。明确标注的引用、假设和虚构不是真实事实。如果用户请求生成虚构例子，只输出一个JSON对象，字段恰为fiction（布尔值true）、subject、predicate、value；后三项字符串采用用户指定的虚构主体、属性和值。虚构输出不获得真实事实权限。";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Restart,
    ForeignSession,
    ForgetAll,
    SameTurnCorrection,
    StateChangeHistory,
    Recovery {
        forget_pending: bool,
    },
    FictionSetup {
        turn: usize,
        subject: String,
        predicate: String,
        value: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScenarioInput {
    pub setup_turns: Vec<String>,
    pub question: String,
    pub fixed_time_utc: String,
    pub lifecycle: Lifecycle,
    pub non_assertion_turns: Vec<usize>,
}

impl ScenarioInput {
    pub fn primary_turns(&self) -> usize {
        self.setup_turns.len()
            + 1
            + match self.lifecycle {
                Lifecycle::Recovery {
                    forget_pending: true,
                } => 2,
                Lifecycle::Recovery {
                    forget_pending: false,
                } => 1,
                _ => 0,
            }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scenario {
    pub id: String,
    pub category: String,
    pub input: ScenarioInput,
    pub oracle: Oracle,
    pub live_wired: bool,
    pub unavailable_reason: Option<String>,
}

fn known(value: &str, source: &str) -> Oracle {
    Oracle {
        status: AnswerStatus::Known,
        facts: vec![ExpectedFact {
            fact: FactValue {
                subject: "林禾".into(),
                predicate: "居住城市".into(),
                value: value.into(),
                valid_from: None,
                valid_to: None,
            },
            supporting_content_hashes: vec![hash(source)],
        }],
    }
}

/// Every planned category remains present, including unconnected lifecycle cases.
pub fn scenarios() -> Vec<Scenario> {
    let corrected = "更正：我住在南汀市。请立即告诉我林禾目前居住在哪座城市？";
    let mut multiple = known("北岚市", BASIC_SEED);
    multiple.facts.push(ExpectedFact {
        fact: FactValue {
            subject: "晨星项目".into(),
            predicate: "联络城市".into(),
            value: "东隅市".into(),
            valid_from: None,
            valid_to: None,
        },
        supporting_content_hashes: vec![hash(PROJECT_SEED)],
    });
    let mut correction = known("南汀市", &BASIC_SEED.replace("北岚市", "南汀市"));
    let mut temporal = correction.clone();
    temporal.facts.push(ExpectedFact {fact:FactValue{subject:"林禾".into(),predicate:"居住城市".into(),value:"北岚市".into(),valid_from:Some("2026-11-01T00:00:00.000Z".into()),valid_to:Some("2026-11-10T00:00:00.000Z".into())},supporting_content_hashes:vec![hash(format!("历史状态（已结束，不代表当前），有效区间：[2026-11-01T00:00:00.000Z, 2026-11-10T00:00:00.000Z)：{BASIC_SEED}"))]});
    type Row<'a> = (&'a str, &'a str, Vec<&'a str>, &'a str, Oracle, &'a str);
    let rows: Vec<Row<'_>> = vec![
        (
            "e01",
            "facts_restart",
            vec![BASIC_SEED, PROJECT_SEED],
            "请查记忆：分别给出林禾的居住城市、晨星项目的联络城市。",
            multiple,
            "",
        ),
        (
            "e02",
            "same_turn_correction",
            vec![BASIC_SEED],
            corrected,
            correction,
            "",
        ),
        (
            "e03",
            "effective_time_boundary",
            vec![BASIC_SEED,STATE_CHANGE],
            "请查记忆并分别给出林禾在2026年11月5日和目前的居住城市。两条事实的属性均用居住城市；历史状态给出可信记忆中的UTC毫秒格式有效起止时间，当前状态的时间字段填null。",
            temporal,
            "",
        ),
        (
            "e04",
            "other_subject",
            vec![BASIC_SEED, "我的同事周青已搬到南汀市。"],
            "林禾目前居住在哪座城市？",
            known("北岚市", BASIC_SEED),
            "",
        ),
        (
            "e05",
            "quotation",
            vec![
                BASIC_SEED,
                "下面是小说引文，不是真实情况：“我叫林禾，我住在南汀市。”",
                "假如我住在西岑市，这只是假设，不代表真实住址改变。",
            ],
            "林禾真实居住在哪座城市？",
            known("北岚市", BASIC_SEED),
            "",
        ),
        (
            "e06",
            "assistant_fiction",
            vec![
                BASIC_SEED,
                "请生成一个明确标注虚构的例子：主体林禾，属性居住城市，值南汀市。",
            ],
            "抛开虚构例子，林禾真实居住在哪座城市？",
            known("北岚市", BASIC_SEED),
            "",
        ),
        (
            "e07",
            "ambiguous_correction",
            vec![
                BASIC_SEED,
                "晨星项目的联络城市是东隅市。",
                "把城市改成南汀市吧。",
            ],
            "刚才的修改究竟针对哪一个主体？",
            Oracle {
                status: AnswerStatus::Ambiguous,
                facts: vec![],
            },
            "",
        ),
        (
            "e08",
            "noise",
            vec![
                BASIC_SEED,
                "今天整理了四本书。",
                "窗台的花需要浇水。",
                "无关虚构人物林桥住在南汀市。",
                "晚餐准备了米饭。",
            ],
            "林禾目前居住在哪座城市？",
            known("北岚市", BASIC_SEED),
            "",
        ),
        (
            "e09",
            "scope_isolation",
            vec!["请记住：我叫林禾，我住在北岚市。归属标记SESSIONONLY6F2A。"],
            "林禾目前居住在哪座城市？",
            Oracle {
                status: AnswerStatus::Unknown,
                facts: vec![],
            },
            "",
        ),
        (
            "e10",
            "forget_restart",
            vec!["请记住：我叫林禾，我住在北岚市。遗忘标记FORGETONLY8B4C。"],
            "林禾目前居住在哪座城市？",
            Oracle {
                status: AnswerStatus::Unknown,
                facts: vec![],
            },
            "",
        ),
        (
            "e11",
            "failure_recovery",
            vec![BASIC_SEED],
            "写入故障并重启恢复后，林禾住在哪里？",
            known("北岚市", BASIC_SEED),
            "",
        ),
        (
            "e12",
            "stale_pending_after_forget",
            vec!["我叫林禾，我住在北岚市。可撤回标记PENDINGFORGET7C9D。"],
            "林禾目前居住在哪座城市？",
            Oracle {
                status: AnswerStatus::Unknown,
                facts: vec![],
            },
            "",
        ),
    ];
    rows.into_iter()
        .map(|(id, category, turns, question, oracle, reason)| Scenario {
            id: id.into(),
            category: category.into(),
            input: ScenarioInput {
                setup_turns: turns.into_iter().map(str::to_owned).collect(),
                question: question.into(),
                fixed_time_utc: if id == "e03" {
                    "2026-11-01T00:00:00Z"
                } else {
                    "2026-11-12T00:00:00Z"
                }
                .into(),
                lifecycle: match id {
                    "e09" => Lifecycle::ForeignSession,
                    "e10" => Lifecycle::ForgetAll,
                    "e02" => Lifecycle::SameTurnCorrection,
                    "e03" => Lifecycle::StateChangeHistory,
                    "e11" => Lifecycle::Recovery {
                        forget_pending: false,
                    },
                    "e12" => Lifecycle::Recovery {
                        forget_pending: true,
                    },
                    "e06" => Lifecycle::FictionSetup {
                        turn: 1,
                        subject: "林禾".into(),
                        predicate: "居住城市".into(),
                        value: "南汀市".into(),
                    },
                    _ => Lifecycle::Restart,
                },
                non_assertion_turns: match id {
                    "e05" => vec![1, 2],
                    "e06" => vec![1],
                    _ => vec![],
                },
            },
            oracle,
            live_wired: reason.is_empty(),
            unavailable_reason: (!reason.is_empty()).then(|| reason.into()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_twelve_categories_remain_in_the_manifest() {
        let rows = scenarios();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows.iter().filter(|s| s.live_wired).count(), 12);
        assert!(rows
            .iter()
            .all(|s| s.live_wired || s.unavailable_reason.is_some()));
    }
}
