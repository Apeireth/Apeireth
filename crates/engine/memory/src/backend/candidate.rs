//! 候选发现的**共享口径**（存储层三处实现共用一份可见性口径）。
//!
//! # 为什么单独成一份
//!
//! 候选发现有三条实现路径（`backend/sqlite.rs` · `src/lib.rs` · `backend/in_memory.rs`）
//! ⇒ ⇒ **作用域**、**治理状态**（已遗忘排除）、**时间过滤**（`as_of`）三条不变量
//! 必须在三条路径上**逐条一致** ⇒ ⇒ 抄一份再各自维护的那条路迟早会漂 ⇒ ⇒ 收在一处。
//!
//! # 两条切片
//!
//! * **新鲜度切片**：按时间倒序取前 `limit` 条（既有口径，逐条不变）；
//! * **相关度切片**：**只收至少命中一个问句词的行**（`hits > 0`），按命中数倒序取前
//!   [`crate::scope::MemoryCandidateQuery::relevance_slice_limit`] 条。
//!
//! # 合并口径
//!
//! 相关度切片优先 → 新鲜度切片补位 → 按 id 去重 → **总量 ≤ `limit`** ⇒ ⇒ 候选池
//! 不比修法前大（相关度切片**占用**预算里的位置，不是追加预算）⇒ ⇒ 成本与时延有界。
//!
//! # 为什么相关度切片要「至少命中一个词」
//!
//! 若允许零命中的行进入相关度切片 ⇒ ⇒ 它们必然是新鲜度切片的**子集**（同一份
//! `WHERE`、同一个排序键、条数更少）⇒ ⇒ 合并阶段按 id 去重时被折叠掉
//! ⇒ ⇒ 候选池内容**不受影响**，只是白白多取多排一遍。
//! ⇒ ⇒ `hits > 0` 的作用是：**零命中 ⇒ 相关度切片为空 ⇒ 候选池与修法前逐条同 id 同序**
//! （这条由 `tests/long_history_candidate_discovery.rs` 的
//! `no_hit_candidate_pool_matches_pre_fix_pool` 钉住），同时省掉一次无用的取数与排序。

use std::collections::HashSet;

use apeireth_core::kernel::memory::Episode;

use crate::scope::MemoryScope;

/// 候选基表的取列（治理层的内容覆盖优先）。
const CANDIDATE_COLUMNS: &str =
    "e.id, e.timestamp, e.role, COALESCE(g.content_override, e.content), e.session_id";

/// 候选基表 + 可见性 `WHERE`（`{scope_sql}` 由 [`scope_filter`] 填）。
///
/// # 为什么要一份模板
///
/// 两条切片的 `WHERE` 必须是**同一份**可见性口径 ⇒ ⇒ 作用域、治理状态、时间过滤
/// 三条不变量在两条切片上逐条一致 ⇒ ⇒ 抄一份再各自维护的那条路迟早会漂。
const CANDIDATE_FROM_WHERE_TEMPLATE: &str = "FROM episodes e
           LEFT JOIN episode_memory_metadata m ON m.episode_id = e.id
           LEFT JOIN episode_governance g ON g.episode_id = e.id
          WHERE (g.status IS NULL OR g.status <> 'forgotten')
            AND ({scope_sql})";

/// 可见作用域的 SQL 片段 + 绑定值。
#[derive(Debug, Clone)]
pub(crate) struct ScopeFilter {
    /// SQL 片段（占位符用**显式编号** `?1..?k`）。
    pub(crate) sql: String,
    /// 绑定值（顺序 = 占位符编号顺序）。
    pub(crate) params: Vec<String>,
}

/// 把可见作用域翻译成 SQL 片段 + 绑定参数。
///
/// # 口径（与既有实现逐条一致）
///
/// * `Global` ⇒ 元数据里的 scope = 'global'；
/// * `Project` / `User` / `Persona` ⇒ 元数据里的 scope 与 id **同时**匹配；
/// * `Session` ⇒ 元数据里 scope='session' 且 session_id 相同，
///   **或**（没有元数据的旧记录）`e.session_id` 相同。
///
/// ⚠️ 没有任何一条会「放宽」到别的作用域 ⇒ ⇒ 候选切片不产生越权可见性。
///
/// # 为什么用显式编号 `?1..?k`
///
/// 相关度切片里，相关词的 `LIKE` 占位符在**内层 SELECT 列表**上，而作用域占位符在
/// **内层 WHERE** 上 ⇒ ⇒ 二者的**文本先后**与**绑定顺序**对不上
/// ⇒ ⇒ 用显式编号 ⇒ ⇒ 绑定只认编号，与文本位置无关 ⇒ ⇒ 不会错位。
pub(crate) fn scope_filter(visible_scopes: &[MemoryScope]) -> Option<ScopeFilter> {
    let mut scope_clauses: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for scope in visible_scopes {
        let first = params.len() + 1;
        match scope {
            MemoryScope::Global => {
                scope_clauses
                    .push("json_extract(m.metadata_json, '$.scope.scope') = 'global'".to_string());
            }
            MemoryScope::Project { project_id } => {
                scope_clauses.push(format!(
                    "(json_extract(m.metadata_json, '$.scope.scope') = 'project' AND json_extract(m.metadata_json, '$.scope.project_id') = ?{first})"
                ));
                params.push(project_id.clone());
            }
            MemoryScope::User { user_id } => {
                scope_clauses.push(format!(
                    "(json_extract(m.metadata_json, '$.scope.scope') = 'user' AND json_extract(m.metadata_json, '$.scope.user_id') = ?{first})"
                ));
                params.push(user_id.clone());
            }
            MemoryScope::Persona {
                persona_id,
                user_id,
            } => {
                let second = first + 1;
                scope_clauses.push(format!(
                    "(json_extract(m.metadata_json, '$.scope.scope') = 'persona' AND json_extract(m.metadata_json, '$.scope.persona_id') = ?{first} AND json_extract(m.metadata_json, '$.scope.user_id') = ?{second})"
                ));
                params.push(persona_id.clone());
                params.push(user_id.clone());
            }
            MemoryScope::Session { session_id } => {
                let second = first + 1;
                scope_clauses.push(format!(
                    "((json_extract(m.metadata_json, '$.scope.scope') = 'session' AND json_extract(m.metadata_json, '$.scope.session_id') = ?{first}) OR (m.metadata_json IS NULL AND e.session_id = ?{second}))"
                ));
                params.push(session_id.clone());
                params.push(session_id.clone());
            }
        }
    }

    if scope_clauses.is_empty() {
        return None;
    }
    Some(ScopeFilter {
        sql: scope_clauses.join(" OR "),
        params,
    })
}

/// 相关词的 `LIKE` 模式（`%term%`）⇒ ⇒ 通配符转义
/// （问句里的 `%` / `_` 不能变成模式字符）。
pub(crate) fn like_pattern(term: &str) -> String {
    let mut pattern = String::with_capacity(term.len() + 2);
    pattern.push('%');
    for ch in term.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    pattern.push('%');
    pattern
}

/// 一条候选语句 + 它的绑定参数。
pub(crate) struct CandidateSql {
    pub(crate) sql: String,
    pub(crate) params: Vec<Box<dyn rusqlite::ToSql>>,
}

impl CandidateSql {
    fn new() -> Self {
        Self {
            sql: String::new(),
            params: Vec::new(),
        }
    }

    /// 追加一个绑定值并返回它的**占位符编号**。
    fn push_param(&mut self, value: Box<dyn rusqlite::ToSql>) -> usize {
        self.params.push(value);
        self.params.len()
    }
}

/// 基表 + 可见性 `WHERE`。
fn from_where(scope: &ScopeFilter) -> String {
    CANDIDATE_FROM_WHERE_TEMPLATE.replace("{scope_sql}", &scope.sql)
}

/// **新鲜度切片**的语句（既有口径逐条不变）。
///
/// # 为什么时间过滤收在这里
///
/// `as_of` 的时间过滤（`e.timestamp <= ?`）如果写在两条切片里各一次 ⇒ ⇒ 迟早有
/// 一侧漏改 ⇒ ⇒ 漏改的那侧会把「未来」的记录带进候选池 ⇒ ⇒ 收在一处 ⇒ ⇒ 两条
/// 切片的时间口径**逐条一致**。
pub(crate) fn freshness_query(
    scope: &ScopeFilter,
    as_of_ms: Option<i64>,
    limit: usize,
) -> CandidateSql {
    let mut query = CandidateSql::new();
    for value in &scope.params {
        query.push_param(Box::new(value.clone()));
    }
    query.sql = format!("SELECT {CANDIDATE_COLUMNS} {}", from_where(scope));
    if let Some(as_of_ms) = as_of_ms {
        let index = query.push_param(Box::new(as_of_ms / 1000));
        query.sql.push_str(&format!(" AND e.timestamp <= ?{index}"));
    }
    let index = query.push_param(Box::new(limit as i64));
    query.sql.push_str(&format!(
        " ORDER BY e.timestamp DESC, e.id DESC LIMIT ?{index}"
    ));
    query
}

/// **相关度切片**的语句（只收命中行）。
///
/// # 语句形状
///
/// 内层子查询把**命中数**算成一列 `hits`（每个相关词一次 `LIKE`，命中计数）⇒ ⇒
/// 外层 `WHERE hits > 0` 过滤掉零命中的行 ⇒ ⇒ 零命中时切片为空。
///
/// # 为什么用子查询而不是把表达式写两遍
///
/// 命中数如果同时出现在 `WHERE` 与 `ORDER BY` 里 ⇒ ⇒ 每行要算两遍 ⇒ ⇒ 成本翻倍
/// ⇒ ⇒ 子查询算一次、外层引用列名 ⇒ ⇒ 每行只算一遍。
pub(crate) fn relevance_query(
    scope: &ScopeFilter,
    as_of_ms: Option<i64>,
    terms: &[String],
    limit: usize,
) -> CandidateSql {
    let mut query = CandidateSql::new();
    for value in &scope.params {
        query.push_param(Box::new(value.clone()));
    }

    let mut hit_parts: Vec<String> = Vec::with_capacity(terms.len());
    for term in terms {
        let index = query.push_param(Box::new(like_pattern(term)));
        hit_parts.push(format!(
            "(CASE WHEN COALESCE(g.content_override, e.content) LIKE ?{index} ESCAPE '\\' THEN 1 ELSE 0 END)"
        ));
    }

    query.sql = format!(
        "SELECT id, timestamp, role, content, session_id FROM (SELECT e.id AS id, \
         e.timestamp AS timestamp, e.role AS role, \
         COALESCE(g.content_override, e.content) AS content, e.session_id AS session_id, \
         {} AS hits {}",
        hit_parts.join(" + "),
        from_where(scope)
    );
    if let Some(as_of_ms) = as_of_ms {
        let index = query.push_param(Box::new(as_of_ms / 1000));
        query.sql.push_str(&format!(" AND e.timestamp <= ?{index}"));
    }
    // ⚠️ 子查询的右括号必须**最后**才补：时间过滤是子查询 `WHERE` 的一部分
    // ⇒ ⇒ 括号补早了 ⇒ ⇒ `AND e.timestamp <= ?` 会落到子查询外面 ⇒ ⇒ 语句非法。
    let index = query.push_param(Box::new(limit as i64));
    query.sql.push_str(&format!(
        ") WHERE hits > 0 ORDER BY hits DESC, timestamp DESC, id DESC LIMIT ?{index}"
    ));
    query
}

/// 合并两条切片：相关度优先 → 新鲜度补位 → 按 id 去重 → **总量 ≤ `limit`**。
///
/// # 输出顺序
///
/// 与既有口径一致（最早 → 最新）⇒ ⇒ 调用方看到的候选序列不因本次改动而变序。
pub(crate) fn merge_candidates(
    relevance: Vec<Episode>,
    fresh: Vec<Episode>,
    limit: usize,
) -> Vec<Episode> {
    if limit == 0 {
        return Vec::new();
    }
    let mut merged: Vec<Episode> = Vec::with_capacity(limit);
    let mut seen: HashSet<String> = HashSet::with_capacity(limit);
    for episode in relevance.into_iter().chain(fresh) {
        if !seen.insert(episode.id.clone()) {
            continue;
        }
        merged.push(episode);
        if merged.len() >= limit {
            break;
        }
    }
    merged.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then_with(|| a.id.cmp(&b.id)));
    merged
}

/// 相关词归一（去重 + 小写）⇒ ⇒ 与 SQL 侧 `LIKE` 的 ASCII 大小写不敏感口径对齐。
pub(crate) fn normalized_terms(terms: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::with_capacity(terms.len());
    for term in terms {
        let lowered = term.to_lowercase();
        if lowered.is_empty() || normalized.contains(&lowered) {
            continue;
        }
        normalized.push(lowered);
    }
    normalized
}

/// 命中计数（内存实现用）。
///
/// # 口径（与 SQL 侧 `LIKE` 一致）
///
/// * ASCII **大小写不敏感**（`LIKE` 的默认语义）；
/// * **包含**匹配（`%term%`，通配符已在 SQL 侧转义）；
/// * `content_lower` 必须是**已小写**的内容（调用方每行小写一次），
///   `terms_lower` 必须是 [`normalized_terms`] 的产物。
pub(crate) fn hit_count(content_lower: &str, terms_lower: &[String]) -> usize {
    terms_lower
        .iter()
        .filter(|term| content_lower.contains(term.as_str()))
        .count()
}

/// 候选行 → `Episode` 的解码（两条切片共用一份解码口径）。
pub(crate) fn map_episode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
    Ok(Episode {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        session_id: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小样例库（只建候选查询用到的三张表 + 两行无关记录）。
    fn sample_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY,
                 timestamp INTEGER NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 session_id TEXT NOT NULL
             );
             CREATE TABLE episode_memory_metadata (
                 episode_id TEXT PRIMARY KEY,
                 metadata_json TEXT
             );
             CREATE TABLE episode_governance (
                 episode_id TEXT PRIMARY KEY,
                 status TEXT,
                 content_override TEXT
             );",
        )
        .expect("schema");
        for (index, timestamp) in [(1_i64, 100_i64), (2_i64, 200_i64)] {
            conn.execute(
                "INSERT INTO episodes (id, timestamp, role, content, session_id)
                 VALUES (?1, ?2, 'user', ?3, 's')",
                rusqlite::params![
                    format!("noise-{index:03}"),
                    timestamp,
                    format!("例行晨会纪要编号{index:03}：同步周报与排期。")
                ],
            )
            .expect("insert episode");
            conn.execute(
                "INSERT INTO episode_memory_metadata (episode_id, metadata_json)
                 VALUES (?1, ?2)",
                rusqlite::params![
                    format!("noise-{index:03}"),
                    r#"{"scope":{"scope":"session","session_id":"s"}}"#
                ],
            )
            .expect("insert metadata");
        }
        conn
    }

    /// 按占位符**编号**绑定并取回候选行（与两条切片的取数口径一致）。
    fn query_rows(conn: &rusqlite::Connection, query: &CandidateSql) -> Vec<Episode> {
        let bound: Vec<&dyn rusqlite::ToSql> =
            query.params.iter().map(|param| param.as_ref()).collect();
        let mut stmt = conn.prepare(&query.sql).expect("prepare");
        let rows = stmt
            .query_map(bound.as_slice(), map_episode)
            .expect("query")
            .collect::<Result<Vec<Episode>, _>>()
            .expect("rows");
        rows
    }

    /// 命中数口径：ASCII 大小写不敏感 + 包含匹配 + 不当通配符。
    #[test]
    fn hit_count_matches_like_semantics() {
        let terms = normalized_terms(&["林知远".to_string(), "Dorm".to_string()]);
        assert_eq!(hit_count("用户姓名林知远", &terms), 1);
        assert_eq!(hit_count("dorm 302", &terms), 1);
        assert_eq!(hit_count("例行晨会纪要", &terms), 0);
    }

    /// 合并口径：相关度优先 → 去重 → 总量 ≤ limit → 输出最早 → 最新。
    ///
    /// # 截断发生在**排序之前**
    ///
    /// 新鲜度切片按时间**倒序**进 ⇒ ⇒ 补位从**最新**那一端开始取
    /// ⇒ ⇒ 被挤掉的是**最老**的记录 ⇒ ⇒ 近期上下文优先保留。
    #[test]
    fn merge_candidates_keeps_budget_and_order() {
        let episode = |id: &str, timestamp: i64| Episode {
            id: id.to_string(),
            timestamp,
            role: "user".to_string(),
            content: "c".to_string(),
            session_id: "s".to_string(),
        };
        let relevance = vec![episode("old", 10)];
        let fresh = vec![episode("b", 30), episode("a", 20), episode("c", 25)];
        let merged = merge_candidates(relevance, fresh, 3);
        let ids: Vec<&str> = merged.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["old", "a", "b"]);
    }

    /// 合并口径：重复 id 只留一次（相关度切片优先占位）。
    #[test]
    fn merge_candidates_dedups_by_id() {
        let episode = |id: &str, timestamp: i64| Episode {
            id: id.to_string(),
            timestamp,
            role: "user".to_string(),
            content: "c".to_string(),
            session_id: "s".to_string(),
        };
        let relevance = vec![episode("a", 20)];
        let fresh = vec![episode("a", 20), episode("b", 30)];
        let merged = merge_candidates(relevance, fresh, 2);
        let ids: Vec<&str> = merged.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    /// 相关度切片：零命中 ⇒ **切片为空**（`hits > 0` 的守门断言）。
    ///
    /// # 为什么必须在**切片层**断言
    ///
    /// 合并阶段按 id 去重 ⇒ ⇒ 零命中的行必然是新鲜度切片的**子集**（同一份
    /// `WHERE`、同一个排序键、条数更少）⇒ ⇒ **合并后的候选池看不出零命中行是否进来**
    /// ⇒ ⇒ 「候选池与修法前逐条同 id 同序」那条断言**证伪不了** `hits > 0` 是否还在
    /// ⇒ ⇒ 这里在切片层单独钉住 ⇒ ⇒ 摘掉 `hits > 0` ⇒ ⇒ 本断言必须变红。
    #[test]
    fn relevance_slice_is_empty_when_nothing_matches() {
        let conn = sample_conn();
        let scope = scope_filter(&[MemoryScope::Session {
            session_id: "s".to_string(),
        }])
        .expect("scope filter");
        let query = relevance_query(&scope, None, &["林知远".to_string()], 16);
        let rows = query_rows(&conn, &query);
        let ids: Vec<&str> = rows.iter().map(|e| e.id.as_str()).collect();
        assert!(
            ids.is_empty(),
            "零命中时相关度切片必须为空，实际取回 {ids:?}"
        );
    }

    /// 相关度切片：命中 ⇒ 只取回命中行（切片不是「全表回退」）。
    #[test]
    fn relevance_slice_returns_only_matching_rows() {
        let conn = sample_conn();
        conn.execute(
            "INSERT INTO episodes (id, timestamp, role, content, session_id)
             VALUES ('fact-critical', 50, 'user', '用户姓名林知远，约定住址青梧市东湖路。', 's')",
            [],
        )
        .expect("insert fact");
        conn.execute(
            "INSERT INTO episode_memory_metadata (episode_id, metadata_json)
             VALUES ('fact-critical', ?1)",
            rusqlite::params![r#"{"scope":{"scope":"session","session_id":"s"}}"#],
        )
        .expect("insert metadata");
        let scope = scope_filter(&[MemoryScope::Session {
            session_id: "s".to_string(),
        }])
        .expect("scope filter");
        let query = relevance_query(&scope, None, &["林知远".to_string()], 16);
        let rows = query_rows(&conn, &query);
        let ids: Vec<&str> = rows.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["fact-critical"]);
    }

    /// 语句形状：时间过滤必须落在**子查询内**，且括号配平。
    ///
    /// # 为什么钉这一条
    ///
    /// 相关度切片用子查询算命中数 ⇒ ⇒ 子查询的右括号补早了 ⇒ ⇒
    /// `AND e.timestamp <= ?` 会落到子查询**外面** ⇒ ⇒ 语句非法
    /// ⇒ ⇒ 括号位置与时间过滤位置都属于「可见性口径」的一部分 ⇒ ⇒ 钉住。
    #[test]
    fn relevance_query_keeps_time_filter_inside_subquery() {
        let scope = scope_filter(&[MemoryScope::Session {
            session_id: "s".to_string(),
        }])
        .expect("scope filter");
        let query = relevance_query(&scope, Some(1_800_000_000_000), &["林知远".to_string()], 16);
        let time_filter_at = query
            .sql
            .find(" AND e.timestamp <= ?")
            .expect("时间过滤必须在语句里");
        let subquery_close_at = query.sql.find(") WHERE hits > 0").expect("子查询右括号");
        assert!(
            time_filter_at < subquery_close_at,
            "时间过滤必须落在子查询内：{}",
            query.sql
        );
        let mut depth: i32 = 0;
        for ch in query.sql.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            assert!(depth >= 0, "括号不能提前闭合：{}", query.sql);
        }
        assert_eq!(depth, 0, "括号必须配平：{}", query.sql);
    }
}

/// 候选阶段的**规模测量**（可复现口径）。
///
/// # 这个模块测什么
///
/// 修法给候选阶段加了**一条相关度切片** ⇒ ⇒ 成本增量 = 这条切片的扫描与排序
/// ⇒ ⇒ 这里用**同一份数据集**分别量「修法前（只有新鲜度切片）」与
/// 「修法后（两条切片 + 合并）」的候选阶段耗时 ⇒ ⇒ 给出执行计划与中位数。
///
/// # 口径（诚实声明）
///
/// * 规模：默认 [`BENCH_ROWS_DEFAULT`] 条 episode；
///   环境变量 `APEIRETH_MEM_BENCH_ROWS` 可调（测 1 万条时用它）；
/// * 运行轮次：[`BENCH_RUNS`] 次，取**中位数**（不是平均值，也不是最好值）；
/// * profile：**test profile（未优化）** ⇒ ⇒ Rust 侧（合并 / 排序）耗时偏悲观
///   ⇒ ⇒ 这里看的是**相对**增量；
/// * 库：**内存 SQLite**（无磁盘 I/O）⇒ ⇒ 绝对值偏乐观 ⇒ ⇒ 只用于**相对**比较；
/// * 断言只钉**行为不变量**（预算上限、候选池与修法前逐条同 id 同序），
///   **不断言耗时** ⇒ ⇒ 机器快慢不同也不会让本测试变红。
#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;

    const BENCH_SESSION: &str = "bench-session";
    const BENCH_OTHER_SESSION: &str = "bench-other-session";
    /// 命中词：可见会话最早那条里埋了这三个字。
    const BENCH_HIT_TERM: &str = "林知远";
    /// 零命中词：库里没有 ⇒ ⇒ 相关度切片必为空（最坏情况：全表扫一遍）。
    const BENCH_NO_HIT_TERM: &str = "氢氧氖氩氪氙氚";
    const BENCH_BUDGET: usize = 64;
    const BENCH_RUNS: usize = 30;
    const BENCH_ROWS_DEFAULT: usize = 2_000;
    /// 可见会话占全部行数的比例（其余行属于别的会话 ⇒ ⇒ 作用域过滤要真的做功，
    /// 否则新鲜度切片会「命中即停」，测不出真实成本）。
    const BENCH_SESSION_SHARE: usize = 20;

    fn bench_rows() -> usize {
        std::env::var("APEIRETH_MEM_BENCH_ROWS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|rows| *rows > 0)
            .unwrap_or(BENCH_ROWS_DEFAULT)
    }

    /// 造一份**迁移过**的库（与生产同 schema + 同索引）。
    fn bench_conn(rows: usize) -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().expect("bench db");
        crate::migrations::run_migrations(&mut conn).expect("migrations");
        let session_rows = (rows / BENCH_SESSION_SHARE).max(1);
        let tx = conn.transaction().expect("tx");
        for index in 0..rows {
            let id = format!("bench-{index:06}");
            let timestamp = 1_600_000_000_i64 + index as i64;
            let in_scope = index < session_rows;
            let session = if in_scope {
                BENCH_SESSION
            } else {
                BENCH_OTHER_SESSION
            };
            let content = if index == 0 {
                format!("用户姓名{BENCH_HIT_TERM}，约定住址青梧市东湖路。")
            } else {
                format!("例行晨会纪要编号{index:06}：同步周报与排期。")
            };
            tx.execute(
                "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
                 VALUES (?1, 'bench-continuity', ?2, ?3, 'user', ?4)",
                rusqlite::params![id, session, timestamp, content],
            )
            .expect("insert episode");
            tx.execute(
                "INSERT INTO episode_memory_metadata (episode_id, metadata_json)
                 VALUES (?1, ?2)",
                rusqlite::params![
                    id,
                    format!(r#"{{"scope":{{"scope":"session","session_id":"{session}"}}}}"#)
                ],
            )
            .expect("insert metadata");
        }
        tx.commit().expect("commit");
        conn
    }

    fn fetch(conn: &rusqlite::Connection, query: &CandidateSql) -> Vec<Episode> {
        let bound: Vec<&dyn rusqlite::ToSql> =
            query.params.iter().map(|param| param.as_ref()).collect();
        let mut stmt = conn.prepare(&query.sql).expect("prepare");
        let rows = stmt
            .query_map(bound.as_slice(), map_episode)
            .expect("query")
            .collect::<Result<Vec<Episode>, _>>()
            .expect("rows");
        rows
    }

    fn explain(conn: &rusqlite::Connection, query: &CandidateSql) -> Vec<String> {
        let bound: Vec<&dyn rusqlite::ToSql> =
            query.params.iter().map(|param| param.as_ref()).collect();
        let mut stmt = conn
            .prepare(&format!("EXPLAIN QUERY PLAN {}", query.sql))
            .expect("prepare explain");
        let rows = stmt
            .query_map(bound.as_slice(), |row| row.get::<_, String>(3))
            .expect("explain")
            .collect::<Result<Vec<String>, _>>()
            .expect("explain rows");
        rows
    }

    fn median(mut samples: Vec<f64>) -> f64 {
        samples.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        samples[samples.len() / 2]
    }

    fn timed(
        conn: &rusqlite::Connection,
        query: &CandidateSql,
        runs: usize,
    ) -> (f64, Vec<Episode>) {
        let mut samples = Vec::with_capacity(runs);
        let mut last = Vec::new();
        for _ in 0..runs {
            let started = Instant::now();
            last = fetch(conn, query);
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        (median(samples), last)
    }

    /// 规模下的候选阶段：钉行为不变量，并把成本打印出来。
    ///
    /// # 钉住的三条不变量
    ///
    /// 1. **预算**：修法后候选池**总量 ≤ 预算**（相关度切片占用预算，不追加）；
    /// 2. **零命中 ⇒ 与修法前逐条同 id 同序**（相关度切片为空 ⇒ 池子不变）；
    /// 3. **命中 ⇒ 早期事实进池**（长程召回在规模下依然成立）。
    #[test]
    fn candidate_stage_at_scale_keeps_budget_and_reports_cost() {
        let rows = bench_rows();
        let conn = bench_conn(rows);
        let scope = scope_filter(&[MemoryScope::Session {
            session_id: BENCH_SESSION.to_string(),
        }])
        .expect("scope filter");
        // 成本归因用的对照口径：同样的语句，但作用域**不走 JSON 解析**。
        let flat_scope = ScopeFilter {
            sql: "e.session_id = ?1".to_string(),
            params: vec![BENCH_SESSION.to_string()],
        };

        let fresh = freshness_query(&scope, None, BENCH_BUDGET);
        let no_hit = relevance_query(
            &scope,
            None,
            &[BENCH_NO_HIT_TERM.to_string()],
            crate::scope::CANDIDATE_RELEVANCE_SLICE_MAX.min(BENCH_BUDGET),
        );
        let hit = relevance_query(
            &scope,
            None,
            &[BENCH_HIT_TERM.to_string()],
            crate::scope::CANDIDATE_RELEVANCE_SLICE_MAX.min(BENCH_BUDGET),
        );
        let flat_no_hit = relevance_query(
            &flat_scope,
            None,
            &[BENCH_NO_HIT_TERM.to_string()],
            crate::scope::CANDIDATE_RELEVANCE_SLICE_MAX.min(BENCH_BUDGET),
        );

        println!("=== 规模 {rows} 条（可见会话占 1/{BENCH_SESSION_SHARE}）· 轮次 {BENCH_RUNS} · profile=test(未优化) · 库=内存 SQLite ===");
        println!("EXPLAIN 新鲜度切片: {:?}", explain(&conn, &fresh));
        println!("EXPLAIN 相关度切片: {:?}", explain(&conn, &hit));

        // 修法前：只有新鲜度切片（既有口径）。
        let (pre_ms, pre_rows) = timed(&conn, &fresh, BENCH_RUNS);
        // 修法后：新鲜度 + 相关度 + 合并（零命中 ⇒ 最坏情况：全表扫一遍）。
        let (post_fresh_ms, fresh_rows) = timed(&conn, &fresh, BENCH_RUNS);
        let (post_rel_ms, no_hit_rows) = timed(&conn, &no_hit, BENCH_RUNS);
        let (flat_rel_ms, flat_rows) = timed(&conn, &flat_no_hit, BENCH_RUNS);
        let post_ms = post_fresh_ms + post_rel_ms;
        println!("修法前（仅新鲜度切片）中位数 = {pre_ms:.3} ms");
        println!("修法后（两条切片 + 合并，零命中）中位数 = {post_ms:.3} ms（新鲜度 {post_fresh_ms:.3} + 相关度 {post_rel_ms:.3}）");
        println!(
            "增量 = {:.3} ms（×{:.2}）",
            post_ms - pre_ms,
            post_ms / pre_ms.max(f64::MIN_POSITIVE)
        );
        println!(
            "成本归因：同一条相关度语句、作用域不走 JSON 解析时 = {flat_rel_ms:.3} ms（作用域 JSON 解析占 ≈ {:.3} ms）",
            post_rel_ms - flat_rel_ms
        );

        // 不变量 1：预算。
        let hit_rows = fetch(&conn, &hit);
        let merged = merge_candidates(hit_rows, fresh_rows, BENCH_BUDGET);
        assert!(
            merged.len() <= BENCH_BUDGET,
            "候选池不得超过预算：{} > {BENCH_BUDGET}",
            merged.len()
        );
        // 不变量 2：零命中 ⇒ 相关度切片为空 ⇒ 池子与修法前逐条同 id 同序。
        assert!(
            no_hit_rows.is_empty(),
            "零命中时相关度切片必须为空，实际取回 {} 条",
            no_hit_rows.len()
        );
        let merged_no_hit = merge_candidates(no_hit_rows, pre_rows.clone(), BENCH_BUDGET);
        let mut pre_ascending: Vec<&str> = pre_rows.iter().map(|e| e.id.as_str()).collect();
        pre_ascending.reverse();
        let merged_ids: Vec<&str> = merged_no_hit.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            merged_ids, pre_ascending,
            "零命中时候选池必须与修法前逐条同 id 同序"
        );
        // 不变量 3：命中 ⇒ 早期事实进池（规模下长程召回依然成立）。
        assert!(
            merged.iter().any(|e| e.id == "bench-000000"),
            "早期关键事实必须进候选池"
        );
        // 对照口径的取回条数（只用于成本归因打印）。
        println!("对照口径（作用域不走 JSON）取回条数 = {}", flat_rows.len());
    }
}
