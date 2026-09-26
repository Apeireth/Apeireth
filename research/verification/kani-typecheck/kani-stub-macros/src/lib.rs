//! `#[kani::proof]` 的本地桩: 剥掉 #[kani::*] 辅助属性 (unwind 等),
//! 把函数注册为 #[cfg_attr(test, test)] —— 既能被 `cargo check --tests`
//! 全量类型检查, 也能以默认值输入冒烟执行。

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut kept: Vec<TokenTree> = Vec::new();
    let mut iter = item.into_iter().peekable();
    while let Some(tok) = iter.next() {
        if is_hash(&tok) {
            if let Some(next) = iter.peek() {
                if is_kani_attr(next) {
                    iter.next();
                    continue;
                }
            }
        }
        kept.push(tok);
    }
    let mut out: TokenStream = "#[cfg_attr(test, test)]".parse().unwrap();
    out.extend(kept);
    out
}

fn is_hash(tok: &TokenTree) -> bool {
    matches!(tok, TokenTree::Punct(p) if p.as_char() == '#')
}

fn is_kani_attr(tok: &TokenTree) -> bool {
    let TokenTree::Group(group) = tok else {
        return false;
    };
    if group.delimiter() != Delimiter::Bracket {
        return false;
    }
    let mut inner = group.stream().into_iter();
    let name_is_kani = matches!(inner.next(), Some(TokenTree::Ident(id)) if id.to_string() == "kani");
    let scoped = matches!(inner.next(), Some(TokenTree::Punct(p)) if p.as_char() == ':');
    name_is_kani && scoped
}
