# R251 -- In-Process Pure Pattern Searcher

## Problem
`apeireth-tool-codesearch::ast_grep` module requires `ast-grep` binary on PATH.
For agents running in restricted envs (Windows without ast-grep install, container
without apt-get, sandboxed CI), code search is completely unavailable.

## Solution

### New module: `pure_pattern.rs`
Pure Rust pattern matcher implementing the same `AstSearcher` trait as
`AstGrepSearcher` but with 0 external binary dep.

```rust
pub struct PurePatternSearcher {
    pub extension_filter: Option<String>,  // e.g. Some("rs")
    pub max_file_size: u64,                // 1 MiB default
    pub follow_symlinks: bool,
}

// API:
PurePattern::literal("foo")              // substring match
PurePattern::regex(r"fn\s+(\w+)")?       // regex match
PurePattern::auto("foo_bar")?            // auto-detect (meta chars -> regex)

PurePatternSearcher::search_content(text, &pattern) -> Vec<(line, text)>
PurePatternSearcher::search_file(path, &pattern) -> Vec<AstGrepMatch>
PurePatternSearcher::search_dir(root, &pattern) -> Vec<AstGrepMatch>

impl AstSearcher for PurePatternSearcher {
    fn search(root, pattern, lang) -> Vec<AstGrepMatch>  // via auto-detect
    fn search_with_rule(...) -> Err                     // 0 pretend YAML support
}
```

### Dependency reuse
- `regex` crate (already in workspace)
- `walkdir` (already in workspace)
- `AstSearcher` trait + `AstGrepMatch` struct (already in `ast_grep.rs`)
- 0 new deps added

### Design honesty (O-5)
- Not real AST awareness -- text-only pattern matching
- YAML rule files NOT supported (need real ast-grep binary)
- 0 false claims of "smart parsing"
- Caller can choose: pure_pattern (always works, text-only) vs ast_grep (needs binary, full AST)

## Tests (8 new pass)
- r251_01: literal pattern matches substring (2 lines hit)
- r251_02: regex pattern matches `fn \w+`
- r251_03: auto-detects literal vs regex by meta char presence
- r251_04: search_file returns AstGrepMatch-shaped results (file/start_line/end_line/text)
- r251_05: search_dir with extension filter (only .rs files)
- r251_06: search_with_rule returns unsupported error (0 pretend)
- r251_07: search via AstSearcher trait works end-to-end
- r251_08: max_file_size skips files exceeding limit

## Files
- `crates/apeireth-tool-codesearch/src/pure_pattern.rs` (new, ~250 lines)
- `crates/apeireth-tool-codesearch/src/lib.rs` (add `pub mod pure_pattern;`)

cumulative: ~6361 tests pass.
