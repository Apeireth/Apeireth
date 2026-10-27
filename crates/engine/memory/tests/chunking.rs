//! `chunking` 的测试：把「段落级切分」的三条纪律**钉死**。
//!
//! 重点不在「切得出东西」，而在**纪律不许漂**：
//!
//! * **不丢字**（核心断言）：所有段落拼起来 == 原正文 ⇒ **逐字相同**；
//! * **不造边界**：句子**不跨**段落（除非该句超长 ⇒ 那才允许，且**如实**标 `truncated`）；
//! * **确定性**：同输入两次 ⇒ 逐字相等。
//!
//! 另外把**边界**也钉住（空输入 / 超长句 / 尾段过短 / 空探针 / `max_chars = 0`），
//! 免得边界上 panic 或悄悄改口径。

use apeireth_memory::chunking::{
    chunk_record, chunk_records, competitor_density, stats, verify_lossless, Chunk, ChunkConfig,
};

/// 常用配置（`target = 300` / `max = 400` / `min = 120`）。
fn config() -> ChunkConfig {
    ChunkConfig::new(300, 400, 120)
}

/// 造一段「有足够句子」的正文（每句 9 个字）。
fn sentences(count: usize) -> String {
    let mut out = String::new();
    for index in 0..count {
        out.push_str(&format!("第{index:02}句正文内容"));
        out.push('。');
    }
    out
}

// ============================================================
// chunk_record：基本形态
// ============================================================

#[test]
fn short_text_becomes_a_single_chunk() {
    let body = "這是一段很短的正文。";
    let chunks = chunk_record("r1", body, config());
    assert_eq!(chunks.len(), 1, "短文本 ⇒ 应当只切出一段");
    assert_eq!(chunks[0].text, body, "短文本 ⇒ 段落正文 == 原文");
    assert_eq!(chunks[0].record_id, "r1");
    assert_eq!(chunks[0].index, 0);
    assert_eq!(chunks[0].start_char, 0);
    assert!(!chunks[0].truncated, "短文本 ⇒ 不该被硬切");
}

#[test]
fn long_text_is_split_into_multiple_chunks_within_the_hard_cap() {
    let body = sentences(60);
    let chunks = chunk_record("r1", &body, config());
    assert!(
        chunks.len() > 1,
        "60 句 ⇒ 应当切出多段（实际 {}）",
        chunks.len()
    );
    for chunk in &chunks {
        let length = chunk.text.chars().count();
        assert!(
            length <= config().max_chars,
            "每段都必须 ≤ max_chars（实际 {length} > {}）",
            config().max_chars
        );
    }
}

#[test]
fn chunks_concatenate_to_the_original_text_verbatim() {
    // **核心断言**：不丢字。
    let body = sentences(37);
    let chunks = chunk_record("r1", &body, config());
    let rebuilt: String = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    assert_eq!(rebuilt, body, "所有段落拼起来必须与原文逐字相同");
    verify_lossless(&chunks, &body).expect("切分结果必须通过不丢字校验");
}

#[test]
fn start_char_points_at_the_right_place_in_the_original() {
    let body = sentences(29);
    let chunks = chunk_record("r1", &body, config());
    for chunk in &chunks {
        let sliced: String = body
            .chars()
            .skip(chunk.start_char)
            .take(chunk.text.chars().count())
            .collect();
        assert_eq!(
            sliced, chunk.text,
            "start_char={} 处必须能取到本段正文",
            chunk.start_char
        );
        // 段落正文必须是原文的**连续切片**（能在原文里找到）。
        assert!(
            body.contains(&chunk.text),
            "段落正文必须是原文的子串（start_char={}）",
            chunk.start_char
        );
    }
    // 段落序号连续（0,1,2,…）。
    for (position, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, position, "段落序号必须连续");
    }
}

#[test]
fn sentences_do_not_cross_chunk_boundaries_unless_hard_cut() {
    let body = sentences(41);
    let chunks = chunk_record("r1", &body, config());
    for chunk in &chunks {
        if chunk.truncated {
            continue; // 硬切过的段落允许在半句处断开。
        }
        let trimmed = chunk.text.trim_end();
        assert!(
            trimmed.ends_with('。')
                || trimmed.ends_with('！')
                || trimmed.ends_with('？')
                || trimmed.ends_with('；'),
            "未硬切的段落必须以断句符收尾（实际 {:?}）",
            trimmed.chars().last()
        );
    }
}

#[test]
fn an_over_long_sentence_is_hard_cut_and_flagged() {
    // 一句 50 字（> max = 20）⇒ 必须被硬切，且**如实**标 truncated。
    let long_sentence = "超".repeat(50);
    let body = format!("短句。{long_sentence}。");
    let chunks = chunk_record("r1", &body, ChunkConfig::new(10, 20, 5));
    assert!(
        chunks.len() >= 3,
        "超长句 ⇒ 至少切出三段（实际 {}）",
        chunks.len()
    );
    assert!(
        chunks.iter().any(|chunk| chunk.truncated),
        "被硬切过的段落必须标 truncated = true"
    );
    for chunk in &chunks {
        assert!(
            chunk.text.chars().count() <= 20,
            "硬切后每段仍必须 ≤ max_chars"
        );
    }
    // 硬切**不丢字**。
    let rebuilt: String = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    assert_eq!(rebuilt, body, "硬切也必须不丢字");
    verify_lossless(&chunks, &body).expect("硬切结果必须通过不丢字校验");
}

#[test]
fn a_short_tail_chunk_is_merged_into_the_previous_one() {
    // 造一个「前面够长（≈306 字）、尾巴只有 2 字」的正文。
    // 尾巴单独成段时 < min_chars(120) ⇒ **并入前一段** ⇒ 最终只剩 1 段。
    let mut body = sentences(34);
    body.push_str("尾。");
    let chunks = chunk_record("r1", &body, ChunkConfig::new(300, 400, 120));
    assert_eq!(
        chunks.len(),
        1,
        "短尾段必须并入前一段（实际 {} 段：{:?}）",
        chunks.len(),
        chunks
            .iter()
            .map(|chunk| chunk.text.chars().count())
            .collect::<Vec<_>>()
    );
    assert_eq!(chunks[0].text, body, "合并后段落正文 == 原文");
    verify_lossless(&chunks, &body).expect("合并后仍必须不丢字");
}

#[test]
fn the_hard_cap_wins_over_the_soft_floor_when_merging_would_overflow() {
    // **硬上限优先于硬下限**：前一段 306 字 + 尾段 10 字 = 316 > max(310)
    // ⇒ **不并**（宁可留一个短段，也不越上限）⇒ 这是**如实**的取舍。
    let mut body = sentences(34);
    body.push_str("尾巴有十個字長。");
    let chunks = chunk_record("r1", &body, ChunkConfig::new(300, 310, 120));
    assert_eq!(chunks.len(), 2, "并不下 ⇒ 尾段自成一段");
    assert!(
        chunks[1].text.chars().count() < 120,
        "尾段确实短于 min_chars（这是硬上限优先的**已知代价**）"
    );
    for chunk in &chunks {
        assert!(
            chunk.text.chars().count() <= 310,
            "每段都必须 ≤ max_chars（硬上限不可越）"
        );
    }
    let rebuilt: String = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    assert_eq!(rebuilt, body, "不并也不许丢字");
}

#[test]
fn chunking_is_deterministic() {
    let body = sentences(53);
    let first = chunk_record("r1", &body, config());
    let second = chunk_record("r1", &body, config());
    assert_eq!(first, second, "同输入两次 ⇒ 逐字相等");
}

#[test]
fn empty_input_returns_an_empty_vec_without_panicking() {
    let chunks = chunk_record("r1", "", config());
    assert!(chunks.is_empty(), "空正文 ⇒ 空 Vec");
    assert!(stats(&chunks).chunks == 0);
}

#[test]
fn zero_max_chars_does_not_hang_or_panic() {
    // `max_chars = 0` 会让「任何句子都装不下」⇒ 必须兜底（每段 1 字）而不是死循环。
    let body = "一二三。";
    let chunks = chunk_record("r1", body, ChunkConfig::new(0, 0, 0));
    assert!(!chunks.is_empty(), "兜底口径：仍要切出段落");
    let rebuilt: String = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    assert_eq!(rebuilt, body, "兜底口径仍不许丢字");
}

// ============================================================
// chunk_records：跨记录连续
// ============================================================

#[test]
fn chunk_records_keeps_index_continuous_across_records() {
    let records = vec![
        ("a-1".to_string(), sentences(60)),
        ("a-2".to_string(), sentences(40)),
        ("a-3".to_string(), "短。".to_string()),
    ];
    let chunks = chunk_records(&records, config());
    assert!(
        chunks.len() > records.len(),
        "多条长记录 ⇒ 段落数应多于记录数（实际 {} 段 / {} 条记录）",
        chunks.len(),
        records.len()
    );
    for (position, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, position, "段落序号必须跨记录连续");
    }
    // `record_id` 正确。
    for chunk in &chunks {
        assert!(
            records.iter().any(|(id, _)| *id == chunk.record_id),
            "record_id 必须来自输入记录"
        );
    }
    // 每条记录独立切分 ⇒ 段落不跨记录：按 record_id 分组后逐条校验不丢字。
    for (id, body) in &records {
        let owned: Vec<&Chunk> = chunks
            .iter()
            .filter(|chunk| &chunk.record_id == id)
            .collect();
        let rebuilt: String = owned.iter().map(|chunk| chunk.text.as_str()).collect();
        assert_eq!(&rebuilt, body, "记录 {id} 的段落拼起来必须逐字等于它的正文");
    }
}

// ============================================================
// verify_lossless：如实报告
// ============================================================

#[test]
fn verify_lossless_accepts_a_faithful_split() {
    let body = sentences(23);
    let chunks = chunk_record("r1", &body, config());
    assert!(verify_lossless(&chunks, &body).is_ok());
}

#[test]
fn verify_lossless_rejects_a_tampered_chunk() {
    let body = sentences(23);
    let mut chunks = chunk_record("r1", &body, config());
    // 篡改一个字。
    let first = &mut chunks[0];
    let mut chars: Vec<char> = first.text.chars().collect();
    chars[0] = '篡';
    first.text = chars.into_iter().collect();
    let error = verify_lossless(&chunks, &body).expect_err("篡改一个字 ⇒ 必须报错");
    assert!(
        !error.is_empty(),
        "错误消息必须**如实**说明差在哪（不许是空串）"
    );
}

#[test]
fn verify_lossless_rejects_a_missing_chunk() {
    let body = sentences(23);
    let mut chunks = chunk_record("r1", &body, config());
    chunks.pop();
    let error = verify_lossless(&chunks, &body).expect_err("少一段 ⇒ 必须报错");
    assert!(!error.is_empty(), "错误消息必须非空");
}

// ============================================================
// stats：汇总
// ============================================================

#[test]
fn stats_of_empty_input_does_not_panic() {
    let summary = stats(&[]);
    assert_eq!(summary.chunks, 0);
    assert_eq!(summary.avg_chars, 0.0);
    assert_eq!(summary.min_chars, 0);
    assert_eq!(summary.max_chars, 0);
    assert_eq!(summary.truncated, 0);
}

#[test]
fn stats_counts_truncated_chunks_correctly() {
    let long_sentence = "超".repeat(50);
    let body = format!("短句。{long_sentence}。");
    let chunks = chunk_record("r1", &body, ChunkConfig::new(10, 20, 5));
    let summary = stats(&chunks);
    assert_eq!(summary.chunks, chunks.len());
    assert_eq!(
        summary.truncated,
        chunks.iter().filter(|chunk| chunk.truncated).count(),
        "truncated 计数必须与段落实际标记一致"
    );
    assert!(summary.truncated > 0, "本例必须有被硬切的段落");
    assert!(summary.max_chars <= 20, "最长段不得超过 max_chars");
    assert!(summary.min_chars <= summary.max_chars);
    assert!(summary.avg_chars > 0.0);
}

// ============================================================
// competitor_density：主体竞争者密度
// ============================================================

#[test]
fn competitor_density_counts_chunks_containing_the_subject() {
    let records = vec![
        ("a-1".to_string(), "寶玉在這裡。".to_string()),
        ("a-2".to_string(), "黛玉在那裡。".to_string()),
        ("a-3".to_string(), "又有寶玉出現。".to_string()),
    ];
    let chunks = chunk_records(&records, ChunkConfig::new(5, 10, 1));
    assert_eq!(
        competitor_density(&chunks, "寶玉"),
        2,
        "两段提到寶玉 ⇒ 密度 = 2"
    );
    assert_eq!(competitor_density(&chunks, "黛玉"), 1);
    assert_eq!(
        competitor_density(&chunks, "薛寶釵"),
        0,
        "没人提到 ⇒ 密度 = 0"
    );
}

#[test]
fn competitor_density_handles_edge_positions_and_empty_probe() {
    let records = vec![
        ("a-1".to_string(), "主體在段首。".to_string()),
        (
            "a-2".to_string(),
            "這一段沒有那個詞，但主體在段尾主體。".to_string(),
        ),
    ];
    let chunks = chunk_records(&records, ChunkConfig::new(8, 12, 1));
    assert!(
        competitor_density(&chunks, "主體") >= 1,
        "段首/段尾出现都要算"
    );
    assert_eq!(
        competitor_density(&chunks, ""),
        0,
        "空探针 ⇒ 0（空串被任何文本包含 ⇒ 不能照字面统计）"
    );
}
