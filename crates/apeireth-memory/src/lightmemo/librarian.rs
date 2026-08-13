//! Librarian: library-style categorization of memory items.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Category {
    pub name: String,
    pub parent: Option<String>,
}

impl Category {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), parent: None }
    }
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }
}

pub struct Librarian {
    /// category_name → Category
    categories: HashMap<String, Category>,
    /// item_id → category_name
    items: HashMap<String, String>,
}

impl Librarian {
    pub fn new() -> Self {
        Self { categories: HashMap::new(), items: HashMap::new() }
    }

    /// Register a category. Returns false if already exists.
    pub fn register_category(&mut self, cat: Category) -> bool {
        if self.categories.contains_key(&cat.name) {
            return false;
        }
        self.categories.insert(cat.name.clone(), cat);
        true
    }

    /// Categorize an item.
    pub fn categorize(&mut self, item_id: &str, category_name: &str) -> bool {
        if !self.categories.contains_key(category_name) {
            return false;
        }
        self.items.insert(item_id.to_string(), category_name.to_string());
        true
    }

    /// List items in a category.
    pub fn items_in(&self, category_name: &str) -> Vec<String> {
        self.items.iter()
            .filter(|(_, cat)| cat == &category_name)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// List all categories as a tree (BTreeMap for stable ordering).
    pub fn category_tree(&self) -> BTreeMap<String, Vec<String>> {
        let mut tree: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for cat in self.categories.values() {
            let parent = cat.parent.clone().unwrap_or_else(|| "_root".to_string());
            tree.entry(parent).or_default().push(cat.name.clone());
        }
        tree
    }

    pub fn category_count(&self) -> usize { self.categories.len() }
    pub fn item_count(&self) -> usize { self.items.len() }
}

impl Default for Librarian {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_categorize() {
        let mut l = Librarian::new();
        assert!(l.register_category(Category::new("rust")));
        assert!(l.categorize("item1", "rust"));
        let items = l.items_in("rust");
        assert_eq!(items, vec!["item1"]);
    }

    #[test]
    fn duplicate_register_fails() {
        let mut l = Librarian::new();
        l.register_category(Category::new("rust"));
        assert!(!l.register_category(Category::new("rust")));
    }

    #[test]
    fn categorize_unknown_category_fails() {
        let mut l = Librarian::new();
        assert!(!l.categorize("item", "nonexistent"));
    }

    #[test]
    fn category_tree_includes_parent() {
        let mut l = Librarian::new();
        l.register_category(Category::new("root"));
        l.register_category(Category::new("rust").with_parent("root"));
        l.register_category(Category::new("python").with_parent("root"));
        let tree = l.category_tree();
        let root_children = tree.get("root").expect("root entry");
        assert_eq!(root_children.len(), 2);
    }

    #[test]
    fn empty_librarian() {
        let l = Librarian::new();
        assert_eq!(l.category_count(), 0);
        assert_eq!(l.item_count(), 0);
    }

    #[test]
    fn items_in_unknown_category() {
        let l = Librarian::new();
        assert!(l.items_in("nonexistent").is_empty());
    }
}