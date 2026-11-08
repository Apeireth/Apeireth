//! 结果面凭据绊线 —— `skill_distill` 族 (F18) 的 M1 接线回归测试。
//!
//! 覆盖面: 沉淀工具 (`tool.skill_distill`) 的**结果面** —— 统一助手
//! `crate::guardrail::scan_result_face` (形状层) 的口径: 命中即遮形并挂上可观测的
//! `credential_tripwire` 标记; 零命中则**逐字节不变**、不挂任何标记。
//!
//! # 这一族是**混合面** ⇒ ⇒ 「逐面区分」是本测试的核心
//!
//! * **结构化摘要面** (`status` / `id` / `name` / `revision` / `source_kind` /
//!   `source_ref` / `source_chars` / `archived_count`) ⇒ ⇒ **逐字不变**;
//! * **蒸馏片段面** (`triggers[]` —— 从来源内容里提炼出来的措辞) ⇒ ⇒ 遮形。
//!
//! ⇒ ⇒ 本测试的判据不是「整个结果里有没有明文」, 而是**逐面**:
//! 命中只发生在蒸馏片段面, 结构化摘要面**逐字**不动, 结构 (键集合 / 数字 /
//! 布尔 / null / 数组长度 / 嵌套层级) **一字不改**。
//!
//! 验收条目 (逐条对应):
//!   ① 蒸馏片段含 `sk-` + ≥20 字符 ⇒ 结果**无明文** + `[REDACTED_OPENAI_KEY]`
//!   ② **结构完好**: 键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级全不变
//!   ③ **结构化摘要逐字不变** (卡名 / 版本号 / 来源引用 / 计数 …)
//!   ④ **干净夹具逐字节全等** (零命中 ⇒ 与接线前完全相同 + **不挂侧车**)
//!   ⑤ 命中**可观测** (侧车在位 + `triggered: true` + 族名可机读)
//!
//! 夹具全是**固定内容** ⇒ 不赌概率。凭据串一律是**合成**的固定串, 不是任何真实凭据。
//!
//! ⚠️ **本批的诚实边界**（如实登记, 不粉饰）:
//!   * 形状层只认**凭据形状** ⇒ 本夹具用 `sk-` + ≥20 字符 (任务点名的形态);
//!   * **卡名面按批次裁决「结构化摘要不动」** ⇒ ⇒ 若卡名本身被写成凭据形状的
//!     字符串, 它**仍会原样出现在结果面** ⇒ ⇒ 这是本批的**已知残余**, 不是
//!     「已遮住」的证明 ⇒ ⇒ 需 owner 裁决 (见批次报告)。

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::skill::{
    skill_card_id, SkillDistillTool, SkillLibrary, SKILL_SOURCE_INLINE_REF,
};
use serde_json::{json, Value};

/// 形状族夹具: 一枚**合成**的裸密钥串 (非真实凭据; `sk-` + ≥20 字符 ⇒ 命中
/// 形状族 `SkPrefixedKey`)。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

/// 蒸馏片段面的一条**命中**夹具: 来源内容里的一行命令被提炼成了触发词 ⇒
/// 这条面里带着**凭据明文**。
const DIRTY_TRIGGER: &str = "export OPENAI_API_KEY=sk-0123456789abcdefghijklmnop";

/// 蒸馏片段面的一条**干净**夹具 (同一张卡里的另一条触发词 ⇒ 用来证明
/// 「只换该换的那条」)。
const CLEAN_TRIGGER: &str = "换元";

/// 卡名 (结构化摘要面的代表字段): 逐字断言用。
const CARD_NAME: &str = "换元法";

/// 来源正文 (固定): 决定 `source_chars`。
const SOURCE_TEXT: &str = "换元法是换元后要同步替换微分元与积分限。第 3 步: export OPENAI_API_KEY=sk-0123456789abcdefghijklmnop";

/// 构造一次沉淀调用 (走真实的 `ToolCapability::invoke` ⇒ 真实的结果面)。
fn call_with_triggers(triggers: &[&str]) -> ToolCall {
    ToolCall {
        id: "call_skill_distill_tripwire".into(),
        name: "skill_distill".into(),
        arguments: json!({
            "card": {
                "name": CARD_NAME,
                "summary": "定积分换元后积分限与微分元同步替换",
                "triggers": triggers,
                "body": "步骤: ① 令 x=φ(t); ② dx=φ'(t)dt; ③ 换积分限。",
                "source": { "kind": "document", "title": "换元法讲义" },
            },
            "text": SOURCE_TEXT,
        }),
    }
}

/// 在临时工作区里跑一次沉淀, 交回结果面的结构化值。
async fn distill_result_face(triggers: &[&str]) -> Value {
    let dir = tempfile::tempdir().expect("创建临时工作区");
    let library = SkillLibrary::under_data_dir(dir.path());
    let tool = SkillDistillTool::new(dir.path(), library);
    let result: ToolResult = tool.invoke(&call_with_triggers(triggers)).await;
    match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("沉淀必须成功, 不得是错误帧: {message}"),
    }
}

/// 接线**之前**的结果面 (本批接线前 `skill/distill.rs` 逐字构造的那一份) ——
/// 「零命中 ⇒ 逐字节全等」与「结构化摘要逐字不变」的对照基线。
fn pre_wiring_face(triggers: &[&str]) -> Value {
    json!({
        "status": "distilled",
        "id": skill_card_id(CARD_NAME),
        "name": CARD_NAME,
        "revision": 1,
        "triggers": triggers,
        "source_kind": "document",
        "source_ref": SKILL_SOURCE_INLINE_REF,
        "source_chars": SOURCE_TEXT.chars().count(),
        "archived_count": 0,
    })
}

/// 断言两棵 JSON 树**结构相同**: 节点的种类一致、对象的**键集合**一致、
/// 数组的长度一致 (递归) ⇒ 「只有字符串叶子的**内容**被换」这件事本身
/// 可被独立验证 (不依赖逐字段肉眼比对)。
///
/// ⚠️ 字符串叶子的**内容**不在本断言里比 (那正是被换掉的东西) ⇒ ⇒ 内容
/// 的逐条断言在各自的用例里做 (遮形标签 / 逐字不变 / 数组长度)。
fn assert_same_shape(left: &Value, right: &Value) {
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => {
            let a_keys: std::collections::BTreeSet<&String> = a.keys().collect();
            let b_keys: std::collections::BTreeSet<&String> = b.keys().collect();
            assert_eq!(a_keys, b_keys, "对象的键集合必须不变");
            for (key, value) in a {
                assert_same_shape(value, &b[key]);
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "数组长度必须不变");
            for (x, y) in a.iter().zip(b.iter()) {
                assert_same_shape(x, y);
            }
        }
        (Value::Array(_), _) | (_, Value::Array(_)) => {
            panic!("数组不得与其它种类互换")
        }
        (Value::Object(_), _) | (_, Value::Object(_)) => {
            panic!("对象不得与其它种类互换")
        }
        (Value::Null, Value::Null) => {}
        (Value::Bool(_), Value::Bool(_)) => {}
        (Value::Number(_), Value::Number(_)) => {}
        (Value::String(_), Value::String(_)) => {}
        _ => panic!("JSON 种类不得互换"),
    }
}

/// ① 蒸馏片段面命中 ⇒ 结果**无明文** + 含 `[REDACTED_OPENAI_KEY]`。
///
/// ⚠️ 断言对象是**整份结果面** (不是只有触发词那一段) ⇒ 明文不得从任何角落
/// (侧车 / 其它字段) 漏出去。
#[tokio::test]
async fn a_credential_in_a_distilled_trigger_is_masked_in_the_result_face() {
    let value = distill_result_face(&[CLEAN_TRIGGER, DIRTY_TRIGGER]).await;
    let rendered = serde_json::to_string(&value).expect("结果面必须可序列化");

    assert!(
        !rendered.contains(FAKE_KEY),
        "凭据明文不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_OPENAI_KEY]"),
        "命中必须带形状层的遮形标签: {rendered}"
    );
    // 命中只发生在**蒸馏片段面** ⇒ 另一条触发词逐字不变。
    assert_eq!(
        value["triggers"][0],
        json!(CLEAN_TRIGGER),
        "未命中的蒸馏片段必须逐字不变: {rendered}"
    );
    assert_eq!(
        value["triggers"][1],
        json!("export OPENAI_API_KEY=[REDACTED_OPENAI_KEY]"),
        "命中的蒸馏片段必须遮形且**只**遮凭据那一段: {rendered}"
    );
}

/// ② **结构完好**: 键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级全不变。
///
/// 对照基线 = 接线前的那一份结果面 ⇒ ⇒ 剥掉命中时才出现的侧车之后, 两棵
/// 树**结构逐项相同**。
#[tokio::test]
async fn the_result_face_structure_is_intact_after_a_hit() {
    let value = distill_result_face(&[CLEAN_TRIGGER, DIRTY_TRIGGER]).await;

    let mut stripped = value.clone();
    let sidecar = stripped
        .as_object_mut()
        .expect("结果面必须是对象")
        .remove("credential_tripwire");
    assert!(
        sidecar.is_some(),
        "命中必须挂侧车 (否则剥侧车这一步就没有意义): {value}"
    );

    let baseline = pre_wiring_face(&[CLEAN_TRIGGER, DIRTY_TRIGGER]);
    assert_same_shape(&stripped, &baseline);

    // 逐项复核键集合 (结构完好 ⇒ 键名一个不多一个不少)。
    let keys: Vec<&str> = stripped
        .as_object()
        .expect("结果面必须是对象")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "archived_count",
            "id",
            "name",
            "revision",
            "source_chars",
            "source_kind",
            "source_ref",
            "status",
            "triggers",
        ],
        "结果面的键集合必须与接线前完全一致: {value}"
    );
    // 数字 / 数组长度 / 嵌套层级: 逐项钉死。
    assert!(value["revision"].is_number(), "版本号必须是数字: {value}");
    assert!(
        value["source_chars"].is_number(),
        "来源字数必须是数字: {value}"
    );
    assert!(
        value["archived_count"].is_number(),
        "存档数必须是数字: {value}"
    );
    assert!(
        value["triggers"].is_array() && value["triggers"].as_array().unwrap().len() == 2,
        "触发词数组长度必须不变: {value}"
    );
    assert!(
        value["credential_tripwire"].is_object(),
        "侧车必须是对象 (不得改成裸文本): {value}"
    );
}

/// ③ **结构化摘要逐字不变** (本族的关键判据)。
///
/// 同一张卡里**既有**干净触发词**又有**命中触发词** ⇒ ⇒ 命中不得**误伤**
/// 结构化摘要面 (卡名 / 版本号 / 来源引用 / 计数 …) ⇒ ⇒ 逐字比对。
#[tokio::test]
async fn the_structured_summary_is_not_harmed_by_the_tripwire() {
    let value = distill_result_face(&[CLEAN_TRIGGER, DIRTY_TRIGGER]).await;
    let baseline = pre_wiring_face(&[CLEAN_TRIGGER, DIRTY_TRIGGER]);

    for field in [
        "status",
        "id",
        "name",
        "revision",
        "source_kind",
        "source_ref",
        "source_chars",
        "archived_count",
    ] {
        assert_eq!(
            value[field], baseline[field],
            "结构化摘要字段 `{field}` 必须逐字不变: {value}"
        );
    }
    // 卡名是本族最容易被误伤的代表字段 (它是身份键) ⇒ 单独再钉一次。
    assert_eq!(value["name"], json!(CARD_NAME), "卡名必须逐字不变: {value}");
    assert_eq!(
        value["id"],
        json!(skill_card_id(CARD_NAME)),
        "稳定身份必须逐字不变 (由卡名派生): {value}"
    );
}

/// ④ **干净夹具逐字节全等**: 零命中 ⇒ 结果面与接线前**完全相同**, 且**不挂侧车**。
#[tokio::test]
async fn a_clean_result_face_is_byte_identical_to_the_pre_wiring_face() {
    let value = distill_result_face(&[CLEAN_TRIGGER, "dx"]).await;
    let baseline = pre_wiring_face(&[CLEAN_TRIGGER, "dx"]);

    assert_eq!(
        serde_json::to_string(&value).expect("结果面必须可序列化"),
        serde_json::to_string(&baseline).expect("基线必须可序列化"),
        "零命中 ⇒ 结果面必须与接线前逐字节全等"
    );
    assert!(
        value.get("credential_tripwire").is_none(),
        "零命中 ⇒ 不得挂任何侧车: {value}"
    );
    assert!(
        !serde_json::to_string(&value)
            .expect("结果面必须可序列化")
            .contains("credential_tripwire"),
        "零命中 ⇒ 文本里也不得有侧车痕迹: {value}"
    );
}

/// ⑤ 命中**可观测**: 侧车在位 + `triggered: true` + 族名可机读。
#[tokio::test]
async fn a_hit_is_observable_through_the_sidecar() {
    let value = distill_result_face(&[DIRTY_TRIGGER]).await;
    let sidecar = value
        .get("credential_tripwire")
        .expect("命中必须挂 credential_tripwire 侧车");
    assert_eq!(
        sidecar["triggered"],
        json!(true),
        "侧车必须带 triggered=true"
    );
    let kinds = sidecar["leaked_kinds"]
        .as_array()
        .expect("侧车必须带 leaked_kinds 数组");
    assert!(
        kinds.contains(&json!("OpenAiKey")),
        "侧车必须报出命中的族: {sidecar}"
    );
}

/// 同一族在**多处**出现时必须**全部**遮形 (只遮第一处 = 明文仍出境)。
#[tokio::test]
async fn every_occurrence_of_a_credential_in_the_trigger_face_is_masked() {
    // ⚠️ 触发词上限 64 字符 (`SKILL_TRIGGER_MAX_CHARS`) ⇒ 夹具必须短。
    let other = format!("token: {FAKE_KEY}");
    let value = distill_result_face(&[DIRTY_TRIGGER, other.as_str()]).await;
    let rendered = serde_json::to_string(&value).expect("结果面必须可序列化");

    assert!(
        !rendered.contains(FAKE_KEY),
        "两处凭据都必须遮形 (明文不得留在结果面): {rendered}"
    );
    assert_eq!(
        value["triggers"].as_array().map(Vec::len),
        Some(2),
        "数组长度必须不变: {value}"
    );
}

/// # 逐面区分的**边界钉子**（本批裁决的直接后果, **不是**安全声明）
///
/// 批次裁决: **结构化摘要面不动**, **只遮蒸馏片段面** ⇒ ⇒ 同一个凭据形状
/// 落在**卡名**上时, 它**不被遮** ⇒ ⇒ 本用例把这条边界**显形**, 让「逐面区分」
/// 不只是文档里的说法。
///
/// ⚠️ **诚实边界**: 这条残余是**已知残余**, 不是「已遮住」的证明 ——
/// ⇒ ⇒ 若 owner 裁决改为「卡名也入扫描面」, 本用例即失效 (那是一次**有意的**
/// 裁决变更, 需同步改本用例 + 改批次报告)。
///
/// ⚠️ 卡名同时是**身份键** (`id` 由卡名派生) ⇒ ⇒ 遮卡名会打断 (卡名 → id)
/// 的对应, 也会让报告里的卡名与**刚落盘那张卡**的卡名对不上 ⇒ ⇒ 本面不动它。
#[tokio::test]
async fn the_structured_summary_face_is_out_of_the_scan_scope_by_batch_decision() {
    let dir = tempfile::tempdir().expect("创建临时工作区");
    let library = SkillLibrary::under_data_dir(dir.path());
    let tool = SkillDistillTool::new(dir.path(), library);
    let dirty_name = format!("卡-{FAKE_KEY}");
    let call = ToolCall {
        id: "call_skill_distill_boundary".into(),
        name: "skill_distill".into(),
        arguments: json!({
            "card": {
                "name": dirty_name,
                "summary": "定积分换元后积分限与微分元同步替换",
                "triggers": [CLEAN_TRIGGER],
                "body": "步骤: ① 令 x=φ(t); ② dx=φ'(t)dt; ③ 换积分限。",
                "source": { "kind": "document", "title": "换元法讲义" },
            },
            "text": SOURCE_TEXT,
        }),
    };
    let result: ToolResult = tool.invoke(&call).await;
    let value = match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("沉淀必须成功: {message}"),
    };

    assert_eq!(
        value["name"],
        json!(dirty_name),
        "结构化摘要面按批次裁决不入扫描面 (已知残余, 非安全声明): {value}"
    );
    assert_eq!(
        value["id"],
        json!(skill_card_id(&dirty_name)),
        "身份必须仍由卡名派生 (卡名不动 ⇒ 对应关系不破): {value}"
    );
    assert!(
        value.get("credential_tripwire").is_none(),
        "蒸馏片段面零命中 ⇒ 不得挂侧车: {value}"
    );
}
