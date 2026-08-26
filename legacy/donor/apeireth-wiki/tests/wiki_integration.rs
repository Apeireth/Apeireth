//! Integration tests for apeireth-wiki (post-1.0.0)
//!
//! src/lib.rs 已有 22 #[test]. 这里 (tests/) 加跨 API 集成 + 边界.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_wiki::{FilesystemWiki, WikiBlock, WikiContextBlock, WikiEntry, WikiIndex, WikiStore};

// =============================================================================
// helpers
// =============================================================================

fn mk_wiki() -> (tempfile::TempDir, FilesystemWiki) {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = FilesystemWiki::open(tmp.path()).unwrap();
    (tmp, wiki)
}

const SAMPLE_TOPIC: &str = "---\n\
title: AI Concepts\n\
tags: [ai, ml]\n\
links: [topics/tools.md]\n\
summary: 人工智能基础概念集合.\n\
---\n\
# AI Concepts\n\
Neural networks learn via gradient descent.\n";

const SAMPLE_TOOLS: &str = "---\n\
title: Tools\n\
tags: [tools, editor]\n\
---\n\
# Tools\n\
Editors and IDEs: vscode, vim.\n";

const SAMPLE_NOTES: &str = "---\n\
title: Meeting Notes\n\
tags: [meeting, ai]\n\
---\n\
# Meeting\n\
Discussed token budget.\n";

// =============================================================================
// WikiIndex
// =============================================================================

#[test]
fn index_new_empty() {
    let idx = WikiIndex::new();
    assert_eq!(idx.len(), 0);
    assert!(idx.is_empty());
}

#[test]
fn index_insert_and_len() {
    let mut idx = WikiIndex::new();
    let e = WikiEntry {
        path: "x.md".into(),
        title: "X".into(),
        summary: "".into(),
        tags: vec!["t".into()],
        links: vec![],
        created_ms: 0,
        updated_ms: 0,
    };
    idx.insert(e);
    assert_eq!(idx.len(), 1);
    assert!(!idx.is_empty());
}

#[test]
fn index_paths_by_tag_basic() {
    let mut idx = WikiIndex::new();
    for (path, tag) in [("a.md", "ai"), ("b.md", "ai"), ("c.md", "tools")] {
        idx.insert(WikiEntry {
            path: path.into(),
            title: path.into(),
            summary: "".into(),
            tags: vec![tag.into()],
            links: vec![],
            created_ms: 0,
            updated_ms: 0,
        });
    }
    let ai_paths = idx.paths_by_tag("ai");
    assert_eq!(ai_paths.len(), 2);
    let tools_paths = idx.paths_by_tag("tools");
    assert_eq!(tools_paths.len(), 1);
}

#[test]
fn index_paths_by_tag_dedup() {
    let mut idx = WikiIndex::new();
    let e = WikiEntry {
        path: "x.md".into(),
        title: "X".into(),
        summary: "".into(),
        tags: vec!["t".into(), "t".into()],
        links: vec![],
        created_ms: 0,
        updated_ms: 0,
    };
    idx.insert(e);
    assert_eq!(idx.paths_by_tag("t").len(), 1, "重复 tag 去重");
}

#[test]
fn index_paths_by_tag_unknown() {
    let idx = WikiIndex::new();
    assert!(idx.paths_by_tag("nonexistent").is_empty());
}

#[test]
fn index_serde_roundtrip() {
    let mut idx = WikiIndex::new();
    idx.insert(WikiEntry {
        path: "a.md".into(),
        title: "A".into(),
        summary: "sum".into(),
        tags: vec!["x".into()],
        links: vec!["b.md".into()],
        created_ms: 100,
        updated_ms: 200,
    });
    let s = serde_json::to_string(&idx).unwrap();
    let back: WikiIndex = serde_json::from_str(&s).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back.entries["a.md"].title, "A");
}

// =============================================================================
// WikiEntry
// =============================================================================

#[test]
fn wiki_entry_clone_eq() {
    let a = WikiEntry {
        path: "x".into(),
        title: "X".into(),
        summary: "".into(),
        tags: vec![],
        links: vec![],
        created_ms: 0,
        updated_ms: 0,
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn wiki_entry_serde() {
    let e = WikiEntry {
        path: "x".into(),
        title: "X".into(),
        summary: "sum".into(),
        tags: vec!["a".into()],
        links: vec![],
        created_ms: 100,
        updated_ms: 200,
    };
    let s = serde_json::to_string(&e).unwrap();
    let back: WikiEntry = serde_json::from_str(&s).unwrap();
    assert_eq!(back, e);
}

// =============================================================================
// WikiBlock
// =============================================================================

#[test]
fn wiki_block_directory_constructor() {
    let b = WikiBlock::directory("name", "content");
    assert_eq!(b.name, "name");
    assert_eq!(b.content, "content");
    assert!(b.path.is_none());
}

#[test]
fn wiki_block_detail_constructor() {
    let b = WikiBlock::detail("name", "path.md", "content");
    assert_eq!(b.name, "name");
    assert_eq!(b.path.as_deref(), Some("path.md"));
}

#[test]
fn wiki_block_clone() {
    let a = WikiBlock::detail("n", "p", "c");
    let b = a.clone();
    assert_eq!(a.name, b.name);
    assert_eq!(a.content, b.content);
}

// =============================================================================
// FilesystemWiki lifecycle
// =============================================================================

#[test]
fn wiki_open_creates_dir() {
    let dir = tempfile::tempdir().unwrap();
    let new_root = dir.path().join("new_wiki");
    assert!(!new_root.exists());
    let _wiki = FilesystemWiki::open(&new_root).unwrap();
    assert!(new_root.exists(), "open 应创建 dir");
}

#[test]
fn wiki_root_returns_path() {
    let (_tmp, wiki) = mk_wiki();
    assert!(wiki.root().exists());
}

#[test]
fn wiki_initial_empty() {
    let (_tmp, wiki) = mk_wiki();
    assert_eq!(wiki.snapshot_index().len(), 0);
    assert!(wiki.all_entries().is_empty());
}

#[test]
fn wiki_open_scans_existing_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("topics")).unwrap();
    std::fs::write(tmp.path().join("topics").join("a.md"), SAMPLE_TOPIC).unwrap();
    let wiki = FilesystemWiki::open(tmp.path()).unwrap();
    assert_eq!(wiki.snapshot_index().len(), 1);
    assert!(wiki.get_by_path("topics/a.md").is_some());
}

// =============================================================================
// update + read_file
// =============================================================================

#[test]
fn wiki_update_then_read() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let content = wiki.read_file("a.md").unwrap();
    assert!(content.contains("AI Concepts"));
    assert!(content.contains("Neural networks"));
}

#[test]
fn wiki_update_creates_index_entry() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let entry = wiki.get_by_path("a.md").unwrap();
    assert_eq!(entry.title, "AI Concepts");
    assert_eq!(entry.tags, vec!["ai", "ml"]);
}

#[test]
fn wiki_update_creates_parent_dir() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("deep/nested/path/topic.md", SAMPLE_TOPIC)
        .unwrap();
    assert!(wiki.get_by_path("deep/nested/path/topic.md").is_some());
}

#[test]
fn wiki_update_overwrites() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let v1 = wiki.get_by_path("a.md").unwrap();
    wiki.update(
        "a.md",
        "---\ntitle: Updated Title\ntags: [new]\n---\n# Updated\n",
    )
    .unwrap();
    let v2 = wiki.get_by_path("a.md").unwrap();
    assert_eq!(v2.title, "Updated Title");
    assert_ne!(v1.title, v2.title);
}

#[test]
fn wiki_read_missing_errors() {
    let (_tmp, wiki) = mk_wiki();
    let r = wiki.read_file("nope.md");
    assert!(r.is_err());
}

// =============================================================================
// path validation
// =============================================================================

#[test]
fn wiki_rejects_dotdot() {
    let (_tmp, wiki) = mk_wiki();
    let r = wiki.update("../escape.md", "x");
    assert!(r.is_err());
}

#[test]
fn wiki_rejects_absolute_path() {
    let (_tmp, wiki) = mk_wiki();
    let r = wiki.update("/abs/path.md", "x");
    assert!(r.is_err());
}

#[test]
fn wiki_rejects_empty_path() {
    let (_tmp, wiki) = mk_wiki();
    let r = wiki.update("", "x");
    assert!(r.is_err());
}

// =============================================================================
// search
// =============================================================================

#[test]
fn wiki_search_finds_by_title() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    let hits = wiki.search("Tools", 10);
    assert!(hits.iter().any(|e| e.path == "b.md"));
}

#[test]
fn wiki_search_finds_by_summary() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let hits = wiki.search("人工智能", 10);
    assert!(hits.iter().any(|e| e.path == "a.md"));
}

#[test]
fn wiki_search_case_insensitive() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let hits = wiki.search("ai concepts", 10); // 全小写
    assert!(hits.iter().any(|e| e.path == "a.md"));
}

#[test]
fn wiki_search_respects_max_results() {
    let (_tmp, wiki) = mk_wiki();
    for i in 0..10 {
        wiki.update(
            &format!("n-{i}.md"),
            "---\ntitle: Shared\n---\nshared body\n",
        )
        .unwrap();
    }
    let hits = wiki.search("shared", 3);
    assert_eq!(hits.len(), 3);
}

#[test]
fn wiki_search_no_match() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let hits = wiki.search("zzz_nonexistent_term_zzz", 10);
    assert!(hits.is_empty());
}

// =============================================================================
// get_by_tag
// =============================================================================

#[test]
fn wiki_get_by_tag_multiple() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    wiki.update("c.md", SAMPLE_NOTES).unwrap();
    let ai = wiki.get_by_tag("ai");
    assert_eq!(ai.len(), 2, "a.md + c.md 都有 ai tag");
}

#[test]
fn wiki_get_by_tag_unique() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    let tools = wiki.get_by_tag("tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].path, "b.md");
}

#[test]
fn wiki_get_by_tag_unknown_empty() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    assert!(wiki.get_by_tag("nonexistent").is_empty());
}

// =============================================================================
// all_entries (sorted by path)
// =============================================================================

#[test]
fn wiki_all_entries_sorted() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("z.md", "---\ntitle: Z\n---\n").unwrap();
    wiki.update("a.md", "---\ntitle: A\n---\n").unwrap();
    wiki.update("m.md", "---\ntitle: M\n---\n").unwrap();
    let all = wiki.all_entries();
    let paths: Vec<&str> = all.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, vec!["a.md", "m.md", "z.md"]);
}

// =============================================================================
// directory_block / expand_block
// =============================================================================

#[test]
fn wiki_directory_block_basic() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    let block = wiki.directory_block(2000);
    assert_eq!(block.name, "wiki-directory");
    assert!(block.path.is_none());
    assert!(block.content.contains("# Wiki 目录"));
    assert!(block.content.contains("AI Concepts"));
}

#[test]
fn wiki_directory_block_truncates() {
    let (_tmp, wiki) = mk_wiki();
    for i in 0..10 {
        wiki.update(
            &format!("n-{i}.md"),
            &format!("---\ntitle: Title {i} with extra content\n---\nbody {i}\n"),
        )
        .unwrap();
    }
    let block = wiki.directory_block(400); // 紧预算
    assert!(
        block.content.contains("(更多条目已截断)"),
        "应触发截断: {}",
        block.content
    );
}

#[test]
fn wiki_expand_block_returns_full() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let block = wiki.expand_block("a.md").unwrap();
    assert_eq!(block.name, "wiki-detail");
    assert_eq!(block.path.as_deref(), Some("a.md"));
    assert!(block.content.contains("Neural networks"));
}

#[test]
fn wiki_expand_block_missing_none() {
    let (_tmp, wiki) = mk_wiki();
    assert!(wiki.expand_block("nope.md").is_none());
}

// =============================================================================
// WikiStore trait dispatch
// =============================================================================

#[test]
fn wiki_store_trait_dispatch() {
    let (_tmp, wiki) = mk_wiki();
    let store: &dyn WikiStore = &wiki;
    store.update("a.md", SAMPLE_TOPIC).unwrap();
    assert!(store.get_by_path("a.md").is_some());
    let hits = store.search("AI", 10);
    assert!(!hits.is_empty());
    let all = store.all_entries();
    assert_eq!(all.len(), 1);
    let _ = store.snapshot_index();
}

#[test]
fn wiki_context_block_trait() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let ctx: &dyn WikiContextBlock = &wiki;
    let dir_block = ctx.directory_block(2000);
    assert!(dir_block.content.contains("AI Concepts"));
    let exp_block = ctx.expand_block("a.md").unwrap();
    assert!(exp_block.content.contains("Neural networks"));
    assert!(ctx.expand_block("nope.md").is_none());
}

// =============================================================================
// rebuild_index
// =============================================================================

#[test]
fn wiki_rebuild_picks_up_external_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let mut wiki = FilesystemWiki::open(tmp.path()).unwrap();
    assert_eq!(wiki.snapshot_index().len(), 0);
    std::fs::write(tmp.path().join("external.md"), SAMPLE_TOPIC).unwrap();
    wiki.rebuild_index().unwrap();
    assert_eq!(wiki.snapshot_index().len(), 1);
    assert!(wiki.get_by_path("external.md").is_some());
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_update_tag_search() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    wiki.update("c.md", SAMPLE_NOTES).unwrap();

    let ai = wiki.get_by_tag("ai");
    assert_eq!(ai.len(), 2);

    // 模拟更新改 tag: 删 ai tag
    wiki.update(
        "c.md",
        "---\ntitle: Meeting Notes Updated\ntags: [meeting]\n---\n# Meeting\n",
    )
    .unwrap();
    let ai2 = wiki.get_by_tag("ai");
    assert_eq!(ai2.len(), 1, "c.md 删了 ai tag → 只剩 a.md");

    let meeting = wiki.get_by_tag("meeting");
    assert_eq!(meeting.len(), 1);
}

#[test]
fn integration_search_then_expand() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();

    let hits = wiki.search("AI", 10);
    assert!(!hits.is_empty());

    let ctx: &dyn WikiContextBlock = &wiki;
    for h in &hits {
        let block = ctx.expand_block(&h.path);
        assert!(block.is_some(), "{} 应能 expand", h.path);
    }
}

#[test]
fn integration_write_search_get_by_tag_directory() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();

    // 1. 写 → 搜 → 查 tag → 列目录
    let hits = wiki.search("AI", 10);
    assert!(!hits.is_empty());

    let ai_paths = wiki.get_by_tag("ai");
    assert!(!ai_paths.is_empty());

    let dir = wiki.directory_block(2000);
    assert!(dir.content.contains("AI Concepts"));
}

#[test]
fn integration_update_then_snapshot_index_changed() {
    let (_tmp, wiki) = mk_wiki();
    wiki.update("a.md", SAMPLE_TOPIC).unwrap();
    let idx1 = wiki.snapshot_index();
    let ts1 = idx1.last_updated_ms;

    std::thread::sleep(std::time::Duration::from_millis(2));
    wiki.update("b.md", SAMPLE_TOOLS).unwrap();
    let idx2 = wiki.snapshot_index();
    let ts2 = idx2.last_updated_ms;
    assert_eq!(idx2.len(), 2);
    assert!(ts2 >= ts1, "应递增: {ts2} >= {ts1}");
}
