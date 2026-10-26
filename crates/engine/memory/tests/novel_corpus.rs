//! `novel_corpus` 的契约测试（小说纯文本 → 挖题器的接线面）。
//!
//! 钉住几件事：
//! 1. **解析得对**：回数、回目联、回次都对；
//! 2. **回目联在标题行之后**的形态能兼容（真实文件里就有这种形态）；
//! 3. Gutenberg 元信息与版权尾注**被丢弃**；
//! 4. **硬折行合并成空格**（不是粘连）；
//! 5. **完全确定**：同一文本解析两次 ⇒ 逐字相等；
//! 6. **接线口径**：`id` = `novel-{回次:04}`、`title` = 回次 + 回目联、
//!    `content` 含回目联、`timestamp_ms` = 回次 × 一天；
//! 7. **挖得出题**，题面里出现回目联；
//! 8. **样本渲染**排序与 `limit` 生效；
//! 9. **不 panic**：空文本 / 无章节 ⇒ 空 Vec；
//! 10. **如实报错**：文件不存在 ⇒ `Err(NotFound)`；读得到但没有回 ⇒ `Err(NoChapters)`。
//!
//! ⚠️ **测试不依赖网络、不依赖真实小说文件**：全部用**手写的合成文本**
//! （形状照抄 Gutenberg 格式，含 `*** START/END OF`、折行、回目联两种位置）。
//! **小说正文不入库**（版权 + 体积）⇒ 这里一个字都不抄真书。

use std::path::{Path, PathBuf};

use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::novel_corpus::{
    load_chapters, mine_novel, parse_chapters, render_samples, to_mine_records, Chapter,
    NovelCorpusError,
};

/// 一小段**仿** Gutenberg 格式的合成文本。
///
/// 覆盖了真实文件里的**全部**形态：
/// * `*** START OF` 之前的 Gutenberg 元信息（要被丢弃）；
/// * 回目联**写在标题行里**（第一回）；
/// * 标题行**只有「第X回」**、回目联在**后面的行**（第三回）；
/// * 标题行后有 `-----` 分隔线；
/// * 正文**硬折行**（一行 30 来字）+ **空行**分段；
/// * 正文里有一行**恰好以「第X回」开头**（回述上一回 ⇒ 不得当成新的一回）；
/// * `*** END OF` 之后的版权尾注（要被丢弃），尾注里**故意**放一行「第X回」做对照。
const SAMPLE: &str = "\
The Project Gutenberg eBook of 测试小说

Title: 测试小说
Author: 佚名
Release date: January 12, 2008 [eBook #24264]
Language: Chinese

*** START OF THE PROJECT GUTENBERG EBOOK 测试小说 ***

第一回　甄士隱夢幻識通靈　賈雨村風塵怀閨秀
-----------------------------------------------------------------
此開卷第一回也．作者自云：因曾歷過一番夢幻之后，故將真事隱去，
留與後世話者．故曰\"甄士隱\"云云．此書記了許多人，大約是
賈雨村賈母等人的故事．

　　這一段是第二段，用來驗證空行要留成段落分隔．

第三回

-----------------------------------------------------------------

賈雨村夤緣复舊職　林黛玉拋父進京都

　　卻說雨村忙回頭看時，不是別人，乃是當日同僚一案參革的號張如圭者．
第四回中既將薛家母子在榮府內寄居等事略已表明，此回則暫不能寫矣．

*** END OF THE PROJECT GUTENBERG EBOOK 测试小说 ***

This ebook is for the use of anyone anywhere in the United States.
第二回　被尾注吞掉的回目　不該被解析出來
";

/// 把合成文本写到临时文件（返回路径；临时目录由调用方持有）。
fn write_sample(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("写入临时文件");
    path
}

// ============================================================
// 一、解析
// ============================================================

#[test]
fn parses_chapter_count_and_titles() {
    let chapters = parse_chapters(SAMPLE);

    assert_eq!(chapters.len(), 2, "两回 ⇒ 两条记录");
    assert_eq!(chapters[0].index, 1);
    assert_eq!(
        chapters[1].index, 3,
        "回次取**书里写的回数**（样本里源文本跳了第二回）"
    );
    assert_eq!(
        chapters[0].title, "甄士隱夢幻識通靈　賈雨村風塵怀閨秀",
        "回目联在标题行里 ⇒ 原样取出来（上下句之间是全角空格）"
    );
}

#[test]
fn merges_couplet_that_sits_on_the_next_line() {
    // 真实形态：标题行只有「第X回」，回目联在**后面**的行（中间隔着分隔线）。
    let chapters = parse_chapters(SAMPLE);

    assert_eq!(
        chapters[1].title, "賈雨村夤緣复舊職　林黛玉拋父進京都",
        "回目联在下一行 ⇒ 也要并进回目联"
    );
    assert!(
        !chapters[1].body.contains("林黛玉拋父進京都"),
        "被认成回目联的行 ⇒ 不能再留在正文里"
    );
}

#[test]
fn drops_gutenberg_header_and_footer() {
    let chapters = parse_chapters(SAMPLE);

    assert_eq!(chapters.len(), 2, "尾注里的「第X回」不该被解析出来");
    for chapter in &chapters {
        assert!(
            !chapter.body.contains("Title:"),
            "Gutenberg 元信息必须被丢弃：{}",
            chapter.body
        );
        assert!(
            !chapter.body.contains("被尾注吞掉的回目"),
            "版权尾注必须被丢弃：{}",
            chapter.body
        );
        assert!(
            !chapter.body.contains("START OF"),
            "起始标记必须被丢弃：{}",
            chapter.body
        );
    }
}

#[test]
fn joins_hard_wrapped_lines_with_spaces() {
    let chapters = parse_chapters(SAMPLE);
    let body = &chapters[0].body;

    assert!(
        body.contains("故將真事隱去， 留與後世話者"),
        "硬折行之间要补空格（原文没有这个空格）：{body}"
    );
    assert!(
        !body.contains("故將真事隱去，留與後世話者"),
        "折行**不能**粘连（会造出原文里不存在的串）：{body}"
    );
}

#[test]
fn keeps_blank_lines_as_paragraph_breaks() {
    let chapters = parse_chapters(SAMPLE);
    let body = &chapters[0].body;

    assert!(body.contains("\n\n"), "空行要留成段落分隔：{body}");
    assert!(
        !body.contains('\n') || body.contains("\n\n"),
        "段内不该再有单换行：{body:?}"
    );
}

#[test]
fn does_not_treat_a_body_line_starting_with_chapter_marker_as_a_chapter() {
    // 正文里有一行恰好以「第四回中既將…」开头（回述上一回）⇒ 不能凭空多出一回。
    let chapters = parse_chapters(SAMPLE);

    assert_eq!(chapters.len(), 2, "正文行不该被当成新的一回");
    assert!(
        chapters[1].body.contains("第四回中既將薛家母子"),
        "那行正文必须留在正文里：{}",
        chapters[1].body
    );
}

#[test]
fn skips_chapters_without_body() {
    // 源文件里存在**重复的标题行**（实测《紅樓夢》第四十五回）⇒ 正文为空的那一回跳过。
    let text = "\
*** START OF THE PROJECT GUTENBERG EBOOK X ***

第三回　上联　下联
-----------------------------------------------------------------

第三回　上联　下联
-----------------------------------------------------------------
　　這一次才有正文，夠長可以挖題了．
*** END OF THE PROJECT GUTENBERG EBOOK X ***
";
    let chapters = parse_chapters(text);

    assert_eq!(chapters.len(), 1, "正文为空的回要跳过（重复标题行）");
    assert_eq!(chapters[0].index, 3, "回次取书里写的回数（第三回）");
    assert!(chapters[0].body.contains("這一次才有正文"));
}

#[test]
fn parsing_is_deterministic() {
    let first = parse_chapters(SAMPLE);
    let second = parse_chapters(SAMPLE);

    assert_eq!(first, second, "同一文本解析两次 ⇒ 逐字相等");
}

#[test]
fn empty_and_chapterless_text_yield_empty_vec() {
    assert!(parse_chapters("").is_empty(), "空文本 ⇒ 空 Vec，不 panic");
    assert!(
        parse_chapters("這是一段散文，沒有任何回目。\n第二段。\n").is_empty(),
        "没有章节 ⇒ 空 Vec，不 panic"
    );
    assert!(
        parse_chapters("*** START OF THE PROJECT GUTENBERG EBOOK X ***\n*** END OF X ***")
            .is_empty(),
        "只有 Gutenberg 标记 ⇒ 空 Vec，不 panic"
    );
}

#[test]
fn keeps_traditional_chinese_untouched() {
    // 语料是**繁体**；本模块**不做繁简转换**（不引依赖，也避免转换带来的失真）。
    let chapters = parse_chapters(SAMPLE);
    let body = &chapters[0].body;

    assert!(body.contains("隱去"), "繁体原样保留：{body}");
    assert!(!body.contains("隐去"), "不该出现简体：{body}");
    assert_eq!(
        chapters[0].title, "甄士隱夢幻識通靈　賈雨村風塵怀閨秀",
        "繁体回目联不该被转换"
    );
}

// ============================================================
// 二、接线（回 → 挖题器记录）
// ============================================================

#[test]
fn to_mine_records_uses_deterministic_fields() {
    let chapters = parse_chapters(SAMPLE);

    let records = to_mine_records(&chapters);

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id, "novel-0001", "记录 id = novel-{{回次:04}}");
    assert_eq!(
        records[1].id, "novel-0003",
        "id 用书里的回数（源文件跳了第二回）"
    );
    assert_eq!(
        records[0].title.as_deref(),
        Some("第一回　甄士隱夢幻識通靈　賈雨村風塵怀閨秀"),
        "title = 回次 + 回目联"
    );
    // ⚠️ **可检索文本里不得有回目联**（这条断言原先是反的，是**总指挥的旧契约**）：
    // 题目由回目联生成 ⇒ 若回目联又出现在可检索文本里 ⇒ 检索**字面命中**
    // ⇒ recall@10 恒满分 ⇒ **卷子失效**。
    // ⇒ 回目联只当「提问的把手」（在 `title` 里），不进 `content`。
    assert!(
        !records[0]
            .content
            .contains("甄士隱夢幻識通靈　賈雨村風塵怀閨秀"),
        "回目联不得进可检索文本：{}",
        records[0].content
    );
    assert!(
        !records[0].content.starts_with("第一回"),
        "回次标题行也不得进可检索文本：{}",
        records[0].content
    );
    assert!(
        records[0].content.contains("此開卷第一回也"),
        "content 必须包含正文：{}",
        records[0].content
    );
    assert_eq!(
        records[0].timestamp_ms, 86_400_000,
        "时间戳 = 回次 × 一天（确定的时间线）"
    );
    assert_eq!(records[1].timestamp_ms, 259_200_000, "时间戳 = 回次 × 一天");
    assert_eq!(
        records[0].thread_id, None,
        "小说没有会话概念 ⇒ thread_id = None"
    );
}

#[test]
fn record_title_renders_the_ordinal_in_chinese() {
    // 回次要写成人读得出来的样子（与源文件同形：两位数用「十」，三位数逐位写）。
    let chapters = vec![
        Chapter {
            index: 10,
            title: "上联　下联".to_string(),
            body: "正文".to_string(),
        },
        Chapter {
            index: 104,
            title: "上联　下联".to_string(),
            body: "正文".to_string(),
        },
    ];

    let records = to_mine_records(&chapters);

    assert_eq!(
        records[0].title.as_deref(),
        Some("第十回　上联　下联"),
        "两位数回次用「十」"
    );
    assert_eq!(
        records[1].title.as_deref(),
        Some("第一零四回　上联　下联"),
        "三位数回次逐位写（与源文件同形）"
    );
}

#[test]
fn record_title_falls_back_to_ordinal_when_couplet_is_empty() {
    let chapters = vec![Chapter {
        index: 3,
        title: String::new(),
        body: "正文".to_string(),
    }];

    let records = to_mine_records(&chapters);

    assert_eq!(
        records[0].title.as_deref(),
        Some("第三回"),
        "没有回目联 ⇒ 只用回次"
    );
    assert!(
        !records[0].content.contains("第三回"),
        "没有回目联时 content 就是正文：{}",
        records[0].content
    );
}

// ============================================================
// 三、挖题
// ============================================================

#[test]
fn mine_novel_produces_cases_with_the_couplet_in_the_question() {
    let chapters = parse_chapters(SAMPLE);

    let cases = mine_novel(&chapters, MiningConfig::default());

    assert!(!cases.is_empty(), "小说正文应当挖得出题");
    assert!(
        cases.iter().any(|case| case.query.contains("第三回")),
        "题面里要出现回次（像人话）：{:?}",
        cases
            .iter()
            .map(|case| case.query.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        cases
            .iter()
            .any(|case| case.query.contains("賈雨村夤緣复舊職")),
        "题面里要出现回目联：{:?}",
        cases
            .iter()
            .map(|case| case.query.as_str())
            .collect::<Vec<_>>()
    );
    for case in &cases {
        assert!(!case.expect_ids.is_empty(), "每题至少一条标准答案");
        for id in &case.expect_ids {
            assert!(id.starts_with("novel-"), "标准答案必须指回某一回：{id}");
        }
    }
}

#[test]
fn mined_fragments_are_substrings_of_the_record_content() {
    let chapters = parse_chapters(SAMPLE);
    let records = to_mine_records(&chapters);

    let cases = mine_novel(&chapters, MiningConfig::default());

    assert!(!cases.is_empty());
    for case in &cases {
        assert_eq!(
            case.expect_fragments.len(),
            case.expect_ids.len(),
            "片段与记录 id 一一对应"
        );
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let record = records
                .iter()
                .find(|record| &record.id == id)
                .expect("标准答案指向的回必须存在");
            assert!(
                record.content.contains(fragment.as_str()),
                "片段必须真的在正文里：{fragment}"
            );
        }
    }
}

// ============================================================
// 四、样本渲染
// ============================================================

#[test]
fn render_samples_sorts_by_kind_and_query() {
    let chapters = parse_chapters(SAMPLE);
    let cases = mine_novel(&chapters, MiningConfig::default());

    let rendered = render_samples(&cases, cases.len());
    let lines: Vec<&str> = rendered.lines().collect();

    assert_eq!(lines.len(), cases.len(), "每条一行");
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    assert_eq!(lines, sorted, "样本要按 (kind, query) 排序");
    for line in &lines {
        assert!(
            line.starts_with('['),
            "每行形如 `[kind] query ⇒ expect_ids=[…]`：{line}"
        );
        assert!(line.contains("⇒"), "每行要有题型与标准答案：{line}");
        assert!(line.contains("expect_ids=["), "每行要列标准答案：{line}");
    }
}

#[test]
fn render_samples_honours_the_limit() {
    let chapters = parse_chapters(SAMPLE);
    let cases = mine_novel(&chapters, MiningConfig::default());

    assert_eq!(render_samples(&cases, 0), "", "limit = 0 ⇒ 空串");
    let one = render_samples(&cases, 1);
    assert_eq!(one.lines().count(), 1, "limit = 1 ⇒ 一行");
    let all = render_samples(&cases, cases.len());
    assert_eq!(
        one,
        all.lines().next().unwrap_or_default(),
        "limit 取的是排序后的前 N 条"
    );
    assert!(
        render_samples(&cases, cases.len() + 10).lines().count() <= cases.len(),
        "超出条数 ⇒ 只给已有的那些"
    );
}

#[test]
fn render_samples_is_deterministic() {
    let chapters = parse_chapters(SAMPLE);
    let cases = mine_novel(&chapters, MiningConfig::default());

    assert_eq!(
        render_samples(&cases, 10),
        render_samples(&cases, 10),
        "同一批题渲染两次 ⇒ 逐字相等"
    );
}

// ============================================================
// 五、读文件
// ============================================================

#[test]
fn load_chapters_reads_a_file() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = write_sample(dir.path(), "novel.txt", SAMPLE);

    let chapters = load_chapters(&path).expect("读小说文件");

    assert_eq!(chapters.len(), 2);
    assert_eq!(
        chapters[0].title, "甄士隱夢幻識通靈　賈雨村風塵怀閨秀",
        "读文件与直接解析必须一致"
    );
}

#[test]
fn load_chapters_reports_missing_file() {
    let missing = PathBuf::from("/tmp/apeireth-novel-corpus-根本不存在的文件.txt");

    let error = load_chapters(&missing).expect_err("文件不存在 ⇒ 必须报 Err");

    assert!(
        matches!(error, NovelCorpusError::NotFound { .. }),
        "实际错误：{error:?}"
    );
    assert!(
        error.to_string().contains("不存在"),
        "错误文案要说清是文件不存在：{error}"
    );
}

#[test]
fn load_chapters_reports_file_without_chapters() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = write_sample(dir.path(), "prose.txt", "這是一段散文，沒有任何回目。\n");

    let error = load_chapters(&path).expect_err("没有回 ⇒ 必须报 Err");

    assert!(
        matches!(error, NovelCorpusError::NoChapters),
        "实际错误：{error:?}"
    );
}

#[test]
fn load_chapters_reports_unreadable_file() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = dir.path().join("a-directory");
    std::fs::create_dir_all(&path).expect("创建目录");

    let error = load_chapters(&path).expect_err("读不了 ⇒ 必须报 Err");

    assert!(
        matches!(error, NovelCorpusError::Unreadable { .. }),
        "实际错误：{error:?}"
    );
}

// ============================================================
// 防泄漏：题面来源不得逐字出现在可检索文本里
// ============================================================

#[test]
fn chapter_title_is_not_copied_into_the_searchable_content() {
    // ⚠️ 这条是**测量有效性**的底线：
    // 题目由回目联生成 ⇒ 若回目联又出现在可检索文本里 ⇒ 检索是**字面命中**
    // ⇒ recall@10 恒等于满分 ⇒ **这个卷子测不出任何东西**。
    //
    // ⇒ 可检索文本 = **正文**；回目联只当「提问的把手」。
    let text = "*** START OF THE PROJECT GUTENBERG EBOOK TEST ***\n\
第一回　甄士隱夢幻識通靈　賈雨村風塵怀閨秀\n\
--------------------------------\n\
此開卷第一回也．此處是正文內容．\n\
*** END OF THE PROJECT GUTENBERG EBOOK TEST ***\n";
    let chapters = parse_chapters(text);
    assert_eq!(chapters.len(), 1);
    let records = to_mine_records(&chapters);
    let content = &records[0].content;
    assert!(
        !content.contains("甄士隱夢幻識通靈"),
        "回目联不得被复制进可检索文本：{content}"
    );
    assert!(
        !content.starts_with("第一回"),
        "回次标题行也不得进可检索文本：{content}"
    );
    assert!(
        content.contains("此處是正文內容"),
        "正文必须保留：{content}"
    );
    // 标题仍然**带着**（它是提问的把手）。
    assert!(records[0].title.is_some());
}

#[test]
fn mined_questions_do_not_leak_the_title_into_the_target_content() {
    // 端到端：挖出来的题，其把手不得逐字出现在目标记录的可检索文本里。
    let text = "*** START OF THE PROJECT GUTENBERG EBOOK TEST ***\n\
第一回　甄士隱夢幻識通靈　賈雨村風塵怀閨秀\n\
--------------------------------\n\
此開卷第一回也．甄士隱困於當地．\n\
第二回　賈夫人仙逝揚州城　冷子興演說榮國府\n\
--------------------------------\n\
冷子興演说榮國府．賈雨村風塵怀閨秀之語亦在．\n\
*** END OF THE PROJECT GUTENBERG EBOOK TEST ***\n";
    let chapters = parse_chapters(text);
    let records = to_mine_records(&chapters);
    let cases = mine_novel(&chapters, MiningConfig::default());
    for case in &cases {
        for (index, id) in case.expect_ids.iter().enumerate() {
            let Some(record) = records.iter().find(|record| &record.id == id) else {
                continue;
            };
            let handle = record.title.as_deref().unwrap_or("");
            // 把手去掉「第X回」前缀后的回目联部分。
            let couplet = handle
                .split_once('　')
                .map(|(_, rest)| rest.trim())
                .unwrap_or("");
            if couplet.is_empty() {
                continue;
            }
            assert!(
                !record.content.contains(couplet),
                "题目 [{:?}] 的把手逐字出现在可检索文本里 ⇒ 检索是字面命中 ⇒ 卷子失效",
                case.query
            );
            let _ = index;
        }
    }
}
