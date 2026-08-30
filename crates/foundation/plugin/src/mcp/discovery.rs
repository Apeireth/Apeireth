//! MCP primitive namespace and method dispatch table.
//!
//! Donor: `legacy/donor/apeireth-mcp/src/primitives.rs`.
//!
//! A lookup table, not a registry of live tools. `Implemented` here means
//! "this library has handler primitives for the method", not "a host is
//! serving it". Sampling / Roots / Logging stay `NotImplemented`.

use serde::{Deserialize, Serialize};

/// MCP primitive namespace (spec §Architecture). Hardcoded 7 variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Primitive {
    Initialize,
    Tools,
    Resources,
    Prompts,
    Sampling,
    Roots,
    Logging,
}

/// Compile-time count. Adding a variant requires updating this.
pub const PRIMITIVE_COUNT: usize = 7;

impl Primitive {
    pub fn methods(&self) -> &'static [&'static str] {
        match self {
            Primitive::Initialize => &["initialize", "notifications/initialized"],
            Primitive::Tools => &[
                "tools/list",
                "tools/call",
                "tools/subscribe",
                "tools/unsubscribe",
            ],
            Primitive::Resources => &[
                "resources/list",
                "resources/read",
                "resources/subscribe",
                "resources/unsubscribe",
                "resources/templates/list",
            ],
            Primitive::Prompts => &["prompts/list", "prompts/get"],
            Primitive::Sampling => &["sampling/createMessage"],
            Primitive::Roots => &["roots/list"],
            Primitive::Logging => &["logging/setLevel"],
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Primitive::Initialize => "initialize",
            Primitive::Tools => "tools",
            Primitive::Resources => "resources",
            Primitive::Prompts => "prompts",
            Primitive::Sampling => "sampling",
            Primitive::Roots => "roots",
            Primitive::Logging => "logging",
        }
    }

    pub const ALL: &'static [Primitive] = &[
        Primitive::Initialize,
        Primitive::Tools,
        Primitive::Resources,
        Primitive::Prompts,
        Primitive::Sampling,
        Primitive::Roots,
        Primitive::Logging,
    ];

    pub fn from_method(method: &str) -> Option<Primitive> {
        for p in Self::ALL {
            if p.methods().contains(&method) {
                return Some(*p);
            }
        }
        None
    }

    pub fn method_count(&self) -> usize {
        self.methods().len()
    }

    pub fn has_method(&self, method: &str) -> bool {
        self.methods().contains(&method)
    }

    pub fn all_method_names() -> Vec<&'static str> {
        let mut all = Vec::new();
        for p in Self::ALL {
            all.extend_from_slice(p.methods());
        }
        all
    }
}

/// Methods this library actually has handlers for.
///
/// `tools/list` and `tools/call` are **declared** on [`Primitive::Tools`]
/// (the spec methods exist) but are **not** implemented here: the v2
/// production client in `apeireth-tools::mcp` already owns that subset.
/// Sampling / Roots / Logging and `resources/templates/list` stay
/// `NotImplemented`.
pub const SUPPORTED_METHODS: &[&str] = &[
    "initialize",
    "notifications/initialized",
    "tools/subscribe",
    "tools/unsubscribe",
    "resources/list",
    "resources/read",
    "resources/subscribe",
    "resources/unsubscribe",
    "prompts/list",
    "prompts/get",
];

/// Dispatch decision. `Implemented` ≠ production-wired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveDispatch {
    Implemented(Primitive),
    NotImplemented(Primitive),
    UnknownMethod(String),
}

/// Classify a method name against the library's handler set.
pub fn dispatch_by_method(method: &str) -> PrimitiveDispatch {
    match Primitive::from_method(method) {
        Some(p @ (Primitive::Sampling | Primitive::Roots | Primitive::Logging)) => {
            PrimitiveDispatch::NotImplemented(p)
        }
        Some(Primitive::Resources) if method == "resources/templates/list" => {
            PrimitiveDispatch::NotImplemented(Primitive::Resources)
        }
        Some(p) => {
            if SUPPORTED_METHODS.contains(&method) {
                PrimitiveDispatch::Implemented(p)
            } else {
                PrimitiveDispatch::NotImplemented(p)
            }
        }
        None => PrimitiveDispatch::UnknownMethod(method.to_string()),
    }
}

const _: () = {
    assert!(PRIMITIVE_COUNT == 7);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_enum_exhaustive() {
        assert_eq!(PRIMITIVE_COUNT, 7);
        assert_eq!(Primitive::ALL.len(), 7);
        for p in Primitive::ALL {
            assert!(!p.methods().is_empty());
            assert!(!p.as_str().is_empty());
        }
        assert_eq!(Primitive::from_method("tools/list"), Some(Primitive::Tools));
        assert_eq!(
            Primitive::from_method("resources/list"),
            Some(Primitive::Resources)
        );
        assert_eq!(
            Primitive::from_method("prompts/get"),
            Some(Primitive::Prompts)
        );
        assert_eq!(
            Primitive::from_method("initialize"),
            Some(Primitive::Initialize)
        );
        assert_eq!(Primitive::from_method("unknown/method"), None);
        let names: Vec<&str> = Primitive::ALL.iter().map(|p| p.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[test]
    fn primitive_roundtrip_serde() {
        for p in Primitive::ALL {
            let s = serde_json::to_string(p).unwrap();
            let back: Primitive = serde_json::from_str(&s).unwrap();
            assert_eq!(*p, back);
        }
    }

    #[test]
    fn dispatch_implemented_vs_not() {
        // tools/list + tools/call are owned by the v2 client in
        // `apeireth-tools::mcp`. This library does not grow a second host.
        assert_eq!(
            dispatch_by_method("tools/list"),
            PrimitiveDispatch::NotImplemented(Primitive::Tools)
        );
        assert_eq!(
            dispatch_by_method("tools/call"),
            PrimitiveDispatch::NotImplemented(Primitive::Tools)
        );
        assert_eq!(
            dispatch_by_method("resources/subscribe"),
            PrimitiveDispatch::Implemented(Primitive::Resources)
        );
        assert_eq!(
            dispatch_by_method("prompts/list"),
            PrimitiveDispatch::Implemented(Primitive::Prompts)
        );
        assert_eq!(
            dispatch_by_method("sampling/createMessage"),
            PrimitiveDispatch::NotImplemented(Primitive::Sampling)
        );
        assert_eq!(
            dispatch_by_method("roots/list"),
            PrimitiveDispatch::NotImplemented(Primitive::Roots)
        );
        assert_eq!(
            dispatch_by_method("logging/setLevel"),
            PrimitiveDispatch::NotImplemented(Primitive::Logging)
        );
        assert_eq!(
            dispatch_by_method("resources/templates/list"),
            PrimitiveDispatch::NotImplemented(Primitive::Resources)
        );
        assert_eq!(
            dispatch_by_method("unknown/method"),
            PrimitiveDispatch::UnknownMethod("unknown/method".to_string())
        );
    }

    #[test]
    fn tools_unsubscribe_is_supported() {
        assert!(Primitive::Tools.has_method("tools/unsubscribe"));
        assert!(Primitive::Tools.has_method("tools/list"));
        assert!(Primitive::Tools.has_method("tools/call"));
        assert_eq!(
            dispatch_by_method("tools/unsubscribe"),
            PrimitiveDispatch::Implemented(Primitive::Tools)
        );
        // Declared on the primitive, not hosted by this library.
        assert!(!SUPPORTED_METHODS.contains(&"tools/list"));
        assert!(!SUPPORTED_METHODS.contains(&"tools/call"));
    }

    #[test]
    fn all_method_names_covers_supported() {
        let all = Primitive::all_method_names();
        for m in SUPPORTED_METHODS {
            assert!(all.contains(m), "missing {m}");
        }
    }
}
