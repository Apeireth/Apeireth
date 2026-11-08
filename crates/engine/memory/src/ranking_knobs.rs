//! 检索排序旋钮的 **Host 配置面**（`retrieval.json`）—— 只放「**已量过、默认零变化**」的旋钮。
//!
//! # 为什么是「配置面」而不是环境变量
//!
//! * env 名额已 **150/154** 顶格 ⇒ 下一个旋钮必须**先删一个或改配置项**
//!   （`docs/architecture/contract.toml` 的纪律）；
//! * env 是**进程级全局**的 ⇒ ⇒ 不适合「**按部署**给不同取值」；
//! * 仓里已有**配置面先例**：`mesh.json`（Host 配置、StoredDoc 拒开、失败闭合、缺省不注册）。
//!
//! # 为什么**不做运行中热重载**
//!
//! 热重载 = 会话中途**静默**改排序权重 ⇒ 与治理链/审计口径冲突（行为变更不可解释）；
//! 而我们已有**受治理的运行时调权**（调权引擎：证据驱动、会话维度、有账）。
//! ⇒ 本面**只在启动时读一次** ⇒ 行为变更与进程同生命周期 ⇒ 可解释、可复盘。
//!
//! # 口径（失败闭合，三种结果逐条可复核）
//!
//! * **文件缺失** ⇒ `Ok(None)`：不施加任何旋钮 ⇒ ⇒ **逐字节零行为变化**（这是默认态）；
//! * **文件存在但坏**（畸形 / 串档 / 未来版本 / 未知字段 / 越界取值）⇒ `Err(真因)`：
//!   **不注册也不回落默认值**（坏配置不许被当成「没配置」）；
//! * **文件存在且合法** ⇒ `Ok(Some(body))`：只施加**文件里显式写了**的旋钮。
//!
//! # 两把尺子的实话（写在这里，是为了让读配置的人看见）
//!
//! | 旋钮 | 内部卷（模板题） | 真实查询卷（真人问句） | 默认 |
//! |---|---|---|---|
//! | `first_pos_alpha` | recall Δ 0.0000 · nDCG **−0.0104 ~ −0.0219** | recall **+16.9%**（α=0.10） | **0.0（关）** |
//! | `diversity_lambda` | recall **−5.19% / −2.89%**（λ=0.2） | recall −0.92%，近似重复对 **−41.9%** | **0.0（关）** |
//!
//! ⇒ ⇒ **两个旋钮在两把尺子上方向不一致** ⇒ ⇒ **默认必须关**；
//! ⇒ ⇒ 打开它们的正当理由是「**你在自己的语料上量过**」，不是「文档说它更好」。
//!
//! # 中性表述
//!
//! 只描述**机制**（配置面 / 失败闭合 / 两把尺子），不引用任何外部产品名、文档或代码。

use std::path::Path;

use crate::scope::MemoryRankingConfig;

/// 配置文件名（落在**数据根**下，与其它 Host 配置同处）。
pub const RETRIEVAL_KNOBS_FILE: &str = "retrieval.json";
/// 文档身份（防串档：别的 JSON 不会被当成旋钮配置打开）。
const RETRIEVAL_KNOBS_DOC_NAME: &str = "apeireth.retrieval.knobs";
/// 本构建唯一的格式版本。
const RETRIEVAL_KNOBS_DOC_VERSION: u32 = 1;
/// 取值上限（两个旋钮都按 [0,1] 收；越界 ⇒ 拒开，不夹取、不猜）。
const KNOB_MAX: f64 = 1.0;

/// `retrieval.json` 主体：**只放已量过、默认零变化**的旋钮。
///
/// ⚠️ 这份配置**永远不来自模型载荷、也不来自环境变量**：只有运维/用户手写文件
/// 才能改排序行为。`deny_unknown_fields` ⇒ ⇒ 拼错字段一律**拒开**。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalKnobsBody {
    /// **首命中位置**加成强度（`0` = 关）。
    ///
    /// ⚠️ 两把尺子结论相反（模板卷 nDCG 变差 / 真人卷 recall +16.9%）⇒ ⇒
    /// **默认 0.0**；打开前请在自己的语料上量。
    #[serde(default)]
    pub first_pos_alpha: Option<f64>,
    /// **相关性-多样性折中**强度（`0` = 关）。
    ///
    /// ⚠️ 内部两卷实测是**净损失**（λ=0.2 相对 recall −5.19% / −2.89%）⇒ ⇒
    /// **默认 0.0**；需要「去冗余展示」时才显式给值。
    #[serde(default)]
    pub diversity_lambda: Option<f64>,
}

fn knobs_compat() -> apeireth_core::stored_doc::DocCompat {
    apeireth_core::stored_doc::DocCompat::exact(
        RETRIEVAL_KNOBS_DOC_NAME,
        RETRIEVAL_KNOBS_DOC_VERSION,
    )
}

/// 读**数据根**下的 `retrieval.json`。
///
/// # 三种结果（失败闭合）
///
/// * 文件缺失 ⇒ `Ok(None)`（默认态 ⇒ **零行为变化**）；
/// * 文件存在但坏 ⇒ `Err(真因)`（**不回落默认值**）；
/// * 文件存在且合法 ⇒ `Ok(Some(body))`。
pub fn knobs_from_data_root(data_root: &Path) -> Result<Option<RetrievalKnobsBody>, String> {
    knobs_from_file(&data_root.join(RETRIEVAL_KNOBS_FILE))
}

/// 读指定路径的旋钮配置（语义同 [`knobs_from_data_root`]）。
pub fn knobs_from_file(path: &Path) -> Result<Option<RetrievalKnobsBody>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let doc = apeireth_core::stored_doc::open_single::<RetrievalKnobsBody>(path, &knobs_compat())
        .map_err(|error| format!("{} 拒开: {error}", path.display()))?;
    let body = doc.body;
    if let Some(value) = body.first_pos_alpha {
        check_range(path, "first_pos_alpha", value)?;
    }
    if let Some(value) = body.diversity_lambda {
        check_range(path, "diversity_lambda", value)?;
    }
    Ok(Some(body))
}

/// 取值越界 ⇒ **拒开**（不夹取、不猜、不回落默认）。
fn check_range(path: &Path, field: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=KNOB_MAX).contains(&value) {
        return Err(format!(
            "{} 的 {field} = {value} 不在 [0, {KNOB_MAX}] 内（或不是有限数）⇒ ⇒ **拒开**（不夹取、不回落默认）",
            path.display()
        ));
    }
    Ok(())
}

/// 把旋钮施加到排序配置上（**只施加文件里显式写了的**）。
///
/// ⚠️ 未出现的旋钮**保持调用方传入的值**（例如调权引擎算出来的生效权重）。
pub fn apply_knobs(ranking: &mut MemoryRankingConfig, body: &RetrievalKnobsBody) {
    if let Some(value) = body.first_pos_alpha {
        ranking.first_pos_alpha = value;
    }
    if let Some(value) = body.diversity_lambda {
        ranking.diversity_lambda = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &serde_json::Value) {
        let envelope = serde_json::json!({
            "name": RETRIEVAL_KNOBS_DOC_NAME,
            "version": RETRIEVAL_KNOBS_DOC_VERSION,
            "compatible_versions": [RETRIEVAL_KNOBS_DOC_VERSION],
            "body": body,
        });
        std::fs::write(path, serde_json::to_vec(&envelope).expect("json")).expect("write");
    }

    #[test]
    fn missing_file_is_none_and_changes_nothing() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-knobs-missing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join(RETRIEVAL_KNOBS_FILE);
        let _ = std::fs::remove_file(&path);
        assert!(
            knobs_from_file(&path).expect("read").is_none(),
            "缺文件必须是无旋钮"
        );
        let mut ranking = MemoryRankingConfig::default();
        apply_knobs(&mut ranking, &RetrievalKnobsBody::default());
        assert_eq!(ranking.first_pos_alpha, 0.0);
        assert_eq!(ranking.diversity_lambda, 0.0);
    }

    #[test]
    fn a_legal_file_applies_only_the_written_knobs() {
        let dir = std::env::temp_dir().join(format!("apeireth-knobs-ok-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join(RETRIEVAL_KNOBS_FILE);
        write(&path, &serde_json::json!({ "first_pos_alpha": 0.10 }));
        let body = knobs_from_file(&path).expect("read").expect("some");
        let mut ranking = MemoryRankingConfig::default();
        apply_knobs(&mut ranking, &body);
        assert_eq!(ranking.first_pos_alpha, 0.10, "写了的旋钮要生效");
        assert_eq!(ranking.diversity_lambda, 0.0, "没写的旋钮必须保持原值");
    }

    #[test]
    fn unknown_fields_and_out_of_range_are_refused() {
        let dir = std::env::temp_dir().join(format!("apeireth-knobs-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join(RETRIEVAL_KNOBS_FILE);
        write(&path, &serde_json::json!({ "nope": 1 }));
        assert!(knobs_from_file(&path).is_err(), "未知字段必须拒开");
        write(&path, &serde_json::json!({ "diversity_lambda": 2.0 }));
        assert!(knobs_from_file(&path).is_err(), "越界取值必须拒开");
        write(&path, &serde_json::json!({ "first_pos_alpha": -0.1 }));
        assert!(knobs_from_file(&path).is_err(), "负值必须拒开");
    }
}
