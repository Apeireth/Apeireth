//! Capability **family** roots and segment-boundary membership: one true source.
//!
//! A family is a set of capability ids that share a root but are discovered
//! at runtime, so they cannot be enumerated one by one (the external-tool
//! bridge produces `tool.mcp.<server>.<tool>`). Membership is decided at a
//! **segment boundary**: the root must be followed by a dot, so `tool.mcp`
//! covers `tool.mcp.demo.lookup` but never `tool.mcpsomething` and never the
//! bare root itself.
//!
//! This module is the **single true source** for both the root constant and
//! the membership predicate: the governance layer, the runtime assembly and
//! the bridge itself all read them from here, so the family cannot drift
//! between the authorization surface and the tool-taxonomy surface.

/// Family root of the external-tool (MCP) dynamic capability family.
///
/// Members are `tool.mcp.<server>.<tool>` — see
/// [`crate::mcp_bridge::catalog::capability_id_for`], which composes the id
/// from this root.
pub const MCP_FAMILY_ROOT: &str = "tool.mcp";

/// Whether a capability id belongs to the family rooted at `family_root`.
///
/// Segment boundary: the id must continue the root with a dot.
///
/// * `tool.mcp` (root) covers `tool.mcp.demo.lookup`.
/// * `tool.mcp` does **not** cover `tool.mcpsomething` (same text, different
///   segment).
/// * `tool.mcp` does **not** cover the bare root `tool.mcp` (the root is the
///   family's root, not one of its members).
///
/// No allocation: the prefix is stripped, never formatted.
#[must_use]
pub fn is_capability_in_family(capability_id: &str, family_root: &str) -> bool {
    capability_id
        .strip_prefix(family_root)
        .is_some_and(|rest| rest.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_of_the_mcp_family_are_recognised_by_segment_boundary() {
        assert!(is_capability_in_family(
            "tool.mcp.demo.lookup",
            MCP_FAMILY_ROOT
        ));
        assert!(is_capability_in_family(
            "tool.mcp.demo.lookup-item",
            MCP_FAMILY_ROOT
        ));
        assert!(is_capability_in_family("tool.mcp.dynamic", MCP_FAMILY_ROOT));
    }

    #[test]
    fn the_root_itself_and_textual_prefixes_are_not_members() {
        assert!(!is_capability_in_family("tool.mcp", MCP_FAMILY_ROOT));
        assert!(!is_capability_in_family(
            "tool.mcpsomething",
            MCP_FAMILY_ROOT
        ));
        assert!(!is_capability_in_family("tool.mcp2.demo", MCP_FAMILY_ROOT));
        assert!(!is_capability_in_family("tool.foo", MCP_FAMILY_ROOT));
        assert!(!is_capability_in_family("tool.mcp.demo", "tool."));
    }

    #[test]
    fn the_family_root_constant_is_the_production_id_shape() {
        // 族根必须与生产 id 形态一致：id = 族根 + "." + server + "." + tool。
        let id = crate::mcp_bridge::catalog::capability_id_for("demo", "lookup-item")
            .expect("dynamic capability id is valid");
        assert_eq!(id.as_str(), "tool.mcp.demo.lookup-item");
        assert!(is_capability_in_family(id.as_str(), MCP_FAMILY_ROOT));
    }
}
