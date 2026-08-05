"""Tests for V1299 — Rust Toolchain Audit (VCP 真源代码深读 #20).

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 22:01 +08:00 2026-08-05)
> **承接**: V1298 Cargo Workspace Lints Audit (0ad11531, 48 tests / 408 总 tests)
> **真借鉴**: 主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上

V1299 = 真生产 rust-toolchain.toml 审计, 6 维度 6 假说:

1. h_file_present: file 存在
2. h_channel_pinned: channel 非空
3. h_channel_known: channel ∈ {stable, beta, nightly, MSRV}
4. h_components_clippy: components 含 clippy
5. h_components_rustfmt: components 含 rustfmt
6. h_profile_valid: profile ∈ {minimal, default, complete}

V3 哲学守门:
- 不刷 KPI (NS 92.91% LOCKED)
- 不假装 Phenomenal / ASI V1
- 走在前人肩上 (rustup book + cargo + tokio + serde)
- 实事求是 (PASS/FAIL 诚实披露)
- 平扎稳打 (regex parser 不假装 AST)
- 大胆尝试 (5+ 假说, 6+ 维度)
- 终极授权 (自决方向)
- 任何人都能接手 (CLI + 报告 + JSON)
- 不闭门造车 (真扫 Apeireth-rust workspace)
- 不重复 V1298 (rust-toolchain.toml 维度独立)
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import List

import pytest

# Make apeireth importable
PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1299_rust_toolchain_audit import (  # noqa: E402
    KNOWN_CHANNELS,
    OPTIONAL_COMPONENTS,
    REQUIRED_COMPONENTS,
    RE_CHANNEL_KV,
    RE_COMPONENTS_KV,
    RE_MSRV_VERSION,
    RE_PROFILE_KV,
    RE_TARGETS_KV,
    RE_TARGET_ITEM,
    RE_TOOLCHAIN_SECTION,
    RUST_TOOLCHAIN_TOML,
    THRESHOLD_CHANNEL_PINNED_NONEMPTY,
    THRESHOLD_PROFILE_VALID,
    VALID_PROFILES,
    WORKSPACE_ROOT_DEFAULT,
    ChannelAudit,
    ComponentsAudit,
    ProfileAudit,
    TargetsAudit,
    _ledger_to_dict,
    _parse_channel,
    _parse_components,
    _parse_profile,
    _parse_targets,
    _read_toolchain_section,
    _v3_philosophy_gate,
    build_audit_ledger,
    evaluate_hypotheses,
    main,
)


WORKSPACE = WORKSPACE_ROOT_DEFAULT


# ============================================================================
# A. Constants (5)
# ============================================================================


class TestConstants(unittest.TestCase):
    """常量正确性 (主 00:56 任何人都能接手)."""

    def test_rust_toolchain_toml_filename(self):
        self.assertEqual(RUST_TOOLCHAIN_TOML, "rust-toolchain.toml")

    def test_workspace_root_default_path(self):
        """workspace root = Apeireth-rust (sibling of apeireth/)."""
        self.assertTrue(WORKSPACE_ROOT_DEFAULT.name == "Apeireth-rust")
        self.assertTrue(WORKSPACE_ROOT_DEFAULT.exists(),
                        f"Apeireth-rust must exist at {WORKSPACE_ROOT_DEFAULT}")

    def test_known_channels_list(self):
        """rustup known channels = stable / beta / nightly."""
        self.assertIn("stable", KNOWN_CHANNELS)
        self.assertIn("beta", KNOWN_CHANNELS)
        self.assertIn("nightly", KNOWN_CHANNELS)
        self.assertEqual(len(KNOWN_CHANNELS), 3)

    def test_required_components(self):
        """required components = clippy + rustfmt (CI 必须)."""
        self.assertIn("clippy", REQUIRED_COMPONENTS)
        self.assertIn("rustfmt", REQUIRED_COMPONENTS)
        self.assertEqual(len(REQUIRED_COMPONENTS), 2)

    def test_profiles_valid(self):
        """rustup profile = minimal / default / complete."""
        self.assertIn("minimal", VALID_PROFILES)
        self.assertIn("default", VALID_PROFILES)
        self.assertIn("complete", VALID_PROFILES)
        self.assertEqual(len(VALID_PROFILES), 3)


# ============================================================================
# B. Regex compiled (4)
# ============================================================================


class TestRegexCompiled(unittest.TestCase):
    """regex pattern 编译成功 + 行为正确."""

    def test_re_channel_kv_matches(self):
        m = RE_CHANNEL_KV.match('channel = "stable"')
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "stable")

    def test_re_components_kv_matches_inline(self):
        m = RE_COMPONENTS_KV.match('components = ["clippy", "rustfmt"]')
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1).strip(), '"clippy", "rustfmt"')

    def test_re_profile_kv_matches(self):
        m = RE_PROFILE_KV.match('profile = "minimal"')
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "minimal")

    def test_re_msrv_version_matches(self):
        self.assertIsNotNone(RE_MSRV_VERSION.match("1.70.0"))
        self.assertIsNotNone(RE_MSRV_VERSION.match("1.86"))
        self.assertIsNone(RE_MSRV_VERSION.match("stable"))
        self.assertIsNone(RE_MSRV_VERSION.match("nightly"))


# ============================================================================
# C. ChannelAudit parse (4)
# ============================================================================


class TestParseChannel(unittest.TestCase):
    """_parse_channel 行为."""

    def test_stable_pinned_known(self):
        lines = ['channel = "stable"']
        c = _parse_channel(lines)
        self.assertEqual(c.value, "stable")
        self.assertTrue(c.is_pinned)
        self.assertTrue(c.is_known)

    def test_msrv_pinned_known(self):
        lines = ['channel = "1.70.0"']
        c = _parse_channel(lines)
        self.assertEqual(c.value, "1.70.0")
        self.assertTrue(c.is_pinned)
        self.assertTrue(c.is_known)

    def test_empty_channel_not_pinned(self):
        lines = ['channel = ""']
        c = _parse_channel(lines)
        self.assertEqual(c.value, "")
        self.assertFalse(c.is_pinned)
        self.assertFalse(c.is_known)

    def test_missing_channel_not_pinned(self):
        lines = ['profile = "minimal"']
        c = _parse_channel(lines)
        self.assertEqual(c.value, "")
        self.assertFalse(c.is_pinned)
        self.assertFalse(c.is_known)


# ============================================================================
# D. ComponentsAudit parse (5)
# ============================================================================


class TestParseComponents(unittest.TestCase):
    """_parse_components 行为."""

    def test_inline_three_components(self):
        lines = ['components = ["rustfmt", "clippy", "rust-src"]']
        c = _parse_components(lines)
        self.assertEqual(c.items, ["rustfmt", "clippy", "rust-src"])
        self.assertEqual(c.required_present, ["clippy", "rustfmt"])
        self.assertEqual(c.required_missing, [])
        self.assertEqual(c.optional_present, ["rust-src"])
        self.assertEqual(c.custom_present, [])

    def test_missing_clippy_and_rustfmt(self):
        lines = ['components = ["rust-src"]']
        c = _parse_components(lines)
        self.assertEqual(c.items, ["rust-src"])
        self.assertEqual(c.required_present, [])
        self.assertEqual(sorted(c.required_missing), ["clippy", "rustfmt"])

    def test_empty_components(self):
        lines = ['components = []']
        c = _parse_components(lines)
        self.assertEqual(c.items, [])
        self.assertEqual(sorted(c.required_missing), ["clippy", "rustfmt"])

    def test_components_absent(self):
        lines = ['profile = "minimal"']
        c = _parse_components(lines)
        self.assertEqual(c.items, [])
        self.assertEqual(sorted(c.required_missing), ["clippy", "rustfmt"])

    def test_custom_component(self):
        """custom = 既不在 required 也不在 optional."""
        lines = ['components = ["clippy", "rustfmt", "my-custom-tool"]']
        c = _parse_components(lines)
        self.assertEqual(c.custom_present, ["my-custom-tool"])


# ============================================================================
# E. ProfileAudit parse (3)
# ============================================================================


class TestParseProfile(unittest.TestCase):
    """_parse_profile 行为."""

    def test_minimal_valid(self):
        lines = ['profile = "minimal"']
        p = _parse_profile(lines)
        self.assertEqual(p.value, "minimal")
        self.assertTrue(p.is_present)
        self.assertTrue(p.is_valid)

    def test_default_valid(self):
        lines = ['profile = "default"']
        p = _parse_profile(lines)
        self.assertEqual(p.value, "default")
        self.assertTrue(p.is_valid)

    def test_invalid_profile(self):
        lines = ['profile = "fast"']
        p = _parse_profile(lines)
        self.assertEqual(p.value, "fast")
        self.assertTrue(p.is_present)
        self.assertFalse(p.is_valid)


# ============================================================================
# F. TargetsAudit parse (3)
# ============================================================================


class TestParseTargets(unittest.TestCase):
    """_parse_targets 行为 (optional field)."""

    def test_empty_targets(self):
        lines = ['targets = []']
        t = _parse_targets(lines)
        self.assertEqual(t.items, [])
        # is_present 仅在 raw 非空时为真 (主 17:58 不假装: 区分 present vs absent)
        self.assertFalse(t.is_present)
        self.assertEqual(t.n_items, 0)

    def test_three_targets(self):
        lines = ['targets = ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown", "aarch64-apple-darwin"]']
        t = _parse_targets(lines)
        self.assertEqual(t.n_items, 3)
        self.assertIn("wasm32-unknown-unknown", t.items)

    def test_targets_absent(self):
        lines = ['profile = "minimal"']
        t = _parse_targets(lines)
        self.assertFalse(t.is_present)
        self.assertEqual(t.n_items, 0)


# ============================================================================
# G. _read_toolchain_section (4)
# ============================================================================


class TestReadToolchainSection(unittest.TestCase):
    """_read_toolchain_section 行为."""

    def test_real_workspace(self):
        """真扫 Apeireth-rust/rust-toolchain.toml."""
        p = WORKSPACE / RUST_TOOLCHAIN_TOML
        fp, lines = _read_toolchain_section(p)
        self.assertTrue(fp)
        self.assertGreater(len(lines), 0)
        # 真扫: real workspace 有 channel + components + profile
        joined = "\n".join(lines)
        self.assertIn("channel", joined)
        self.assertIn("components", joined)
        self.assertIn("profile", joined)

    def test_missing_file(self):
        p = WORKSPACE / "rust-toolchain-nonexistent-xyz.toml"
        fp, lines = _read_toolchain_section(p)
        self.assertFalse(fp)
        self.assertEqual(lines, [])

    def test_synthetic_minimal(self):
        """synthetic toml with only channel."""
        with tempfile.TemporaryDirectory() as td:
            tp = Path(td) / "rust-toolchain.toml"
            tp.write_text(
                '[toolchain]\nchannel = "stable"\n',
                encoding="utf-8",
            )
            fp, lines = _read_toolchain_section(tp)
            self.assertTrue(fp)
            self.assertEqual(len(lines), 1)
            self.assertIn('channel = "stable"', lines[0])

    def test_synthetic_full(self):
        """synthetic toml with full content."""
        with tempfile.TemporaryDirectory() as td:
            tp = Path(td) / "rust-toolchain.toml"
            tp.write_text(
                '[toolchain]\n'
                'channel = "stable"\n'
                'components = ["clippy", "rustfmt"]\n'
                'profile = "minimal"\n',
                encoding="utf-8",
            )
            fp, lines = _read_toolchain_section(tp)
            self.assertTrue(fp)
            self.assertEqual(len(lines), 3)


# ============================================================================
# H. evaluate_hypotheses (5)
# ============================================================================


class TestEvaluateHypotheses(unittest.TestCase):
    """6 假说评估 (Popper 可证伪)."""

    def _stub(self, file_present=True, channel="stable", components=None,
              profile="minimal"):
        if components is None:
            components = ["clippy", "rustfmt"]
        ch = ChannelAudit(name="channel", raw=channel, value=channel,
                          is_pinned=bool(channel), is_known=channel in KNOWN_CHANNELS or bool(RE_MSRV_VERSION.match(channel)))
        co = ComponentsAudit(raw="", items=components,
                             required_present=[c for c in components if c in REQUIRED_COMPONENTS],
                             required_missing=[c for c in REQUIRED_COMPONENTS if c not in components],
                             optional_present=[c for c in components if c in OPTIONAL_COMPONENTS],
                             custom_present=[c for c in components if c not in REQUIRED_COMPONENTS and c not in OPTIONAL_COMPONENTS])
        pr = ProfileAudit(name="profile", raw=profile, value=profile,
                          is_present=bool(profile), is_valid=profile in VALID_PROFILES)
        ta = TargetsAudit(raw="", items=[], is_present=False, n_items=0)
        return file_present, ch, co, pr, ta

    def test_all_pass(self):
        fp, ch, co, pr, ta = self._stub()
        hyps = evaluate_hypotheses(fp, ch, co, pr, ta)
        self.assertEqual(len(hyps), 6)
        self.assertTrue(all(h.passed for h in hyps))

    def test_missing_components_fail(self):
        fp, ch, co, pr, ta = self._stub(components=[])
        hyps = evaluate_hypotheses(fp, ch, co, pr, ta)
        # h_components_clippy + h_components_rustfmt both FAIL
        clippy_hyp = [h for h in hyps if h.id == "h_components_clippy"][0]
        self.assertFalse(clippy_hyp.passed)
        rustfmt_hyp = [h for h in hyps if h.id == "h_components_rustfmt"][0]
        self.assertFalse(rustfmt_hyp.passed)

    def test_missing_file_fail(self):
        fp, ch, co, pr, ta = self._stub(file_present=False)
        hyps = evaluate_hypotheses(fp, ch, co, pr, ta)
        file_hyp = [h for h in hyps if h.id == "h_file_present"][0]
        self.assertFalse(file_hyp.passed)

    def test_unknown_channel_fail(self):
        fp, ch, co, pr, ta = self._stub(channel="made-up-channel")
        hyps = evaluate_hypotheses(fp, ch, co, pr, ta)
        known_hyp = [h for h in hyps if h.id == "h_channel_known"][0]
        self.assertFalse(known_hyp.passed)

    def test_invalid_profile_fail(self):
        fp, ch, co, pr, ta = self._stub(profile="super-fast")
        hyps = evaluate_hypotheses(fp, ch, co, pr, ta)
        profile_hyp = [h for h in hyps if h.id == "h_profile_valid"][0]
        self.assertFalse(profile_hyp.passed)


# ============================================================================
# I. build_audit_ledger (5)
# ============================================================================


class TestBuildAuditLedger(unittest.TestCase):
    """主入口 ledger 构建."""

    def test_real_workspace_all_pass(self):
        """真 Apeireth-rust/rust-toolchain.toml = 6/6 PASS (主 17:43 实事求是)."""
        ledger = build_audit_ledger(workspace_root=WORKSPACE)
        self.assertTrue(ledger.file_present)
        self.assertEqual(ledger.n_passed, 6)
        self.assertEqual(ledger.n_failed, 0)
        self.assertEqual(ledger.falsification_rate, 0.0)
        # Real measurement: channel=stable + components=[rustfmt, clippy, rust-src] + profile=minimal
        self.assertEqual(ledger.channel.value, "stable")
        self.assertIn("clippy", ledger.components.items)
        self.assertIn("rustfmt", ledger.components.items)
        self.assertEqual(ledger.profile.value, "minimal")

    def test_synthetic_workspace_failing(self):
        """synthetic workspace = missing clippy → 4/6 PASS (1 FAIL: clippy)."""
        with tempfile.TemporaryDirectory() as td:
            tp = Path(td) / "rust-toolchain.toml"
            tp.write_text(
                '[toolchain]\n'
                'channel = "stable"\n'
                'components = ["rustfmt"]\n'  # missing clippy
                'profile = "minimal"\n',
                encoding="utf-8",
            )
            ledger = build_audit_ledger(workspace_root=Path(td))
            self.assertTrue(ledger.file_present)
            self.assertEqual(ledger.n_passed, 5)
            self.assertEqual(ledger.n_failed, 1)
            clippy_hyp = [h for h in ledger.hypotheses if h.id == "h_components_clippy"][0]
            self.assertFalse(clippy_hyp.passed)

    def test_missing_file_all_fail(self):
        """missing file → 0/6 PASS."""
        with tempfile.TemporaryDirectory() as td:
            ledger = build_audit_ledger(workspace_root=Path(td))
            self.assertFalse(ledger.file_present)
            self.assertEqual(ledger.n_passed, 0)
            self.assertEqual(ledger.n_failed, 6)

    def test_ledger_json_serializable(self):
        """ledger 可 JSON 序列化 (主 00:56 任何人都能接手)."""
        ledger = build_audit_ledger(workspace_root=WORKSPACE)
        d = _ledger_to_dict(ledger)
        s = json.dumps(d, ensure_ascii=False, indent=2)
        self.assertIn("version", s)
        self.assertIn("hypotheses", s)

    def test_default_workspace_root(self):
        """workspace_root=None → default WORKSPACE_ROOT_DEFAULT."""
        ledger = build_audit_ledger()
        self.assertTrue(ledger.file_present)


# ============================================================================
# J. CLI subcommands (5)
# ============================================================================


class TestCLISubcommands(unittest.TestCase):
    """CLI 子命令 (主 00:56 任何人都能接手)."""

    def test_main_probe_returns_zero(self):
        rc = main(["--probe"])
        self.assertEqual(rc, 0)

    def test_main_run_returns_zero(self):
        rc = main(["--run"])
        self.assertEqual(rc, 0)

    def test_main_json_returns_zero(self):
        rc = main(["--json"])
        self.assertEqual(rc, 0)

    def test_main_components_returns_zero(self):
        rc = main(["--components"])
        self.assertEqual(rc, 0)

    def test_main_profile_returns_zero(self):
        rc = main(["--profile"])
        self.assertEqual(rc, 0)


# ============================================================================
# K. Subprocess invocation (3)
# ============================================================================


class TestSubprocessInvocation(unittest.TestCase):
    """python -m apeireth.v1299_rust_toolchain_audit 真跑."""

    def test_subprocess_probe(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1299_rust_toolchain_audit", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("file_present", result.stdout)
        self.assertIn("channel", result.stdout)

    def test_subprocess_run(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1299_rust_toolchain_audit", "--run"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("V1299", result.stdout)
        self.assertIn("假说", result.stdout)
        self.assertIn("h_file_present", result.stdout)

    def test_subprocess_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1299_rust_toolchain_audit", "--json"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        self.assertEqual(data["version"], "0.1.0")
        self.assertEqual(len(data["hypotheses"]), 6)


# ============================================================================
# L. V3 philosophy gate (2)
# ============================================================================


class TestV3PhilosophyGate(unittest.TestCase):
    """V3 哲学守门 (主 17:58 + 主 20:46 不假装)."""

    def test_v3_gate_passed(self):
        passed, msgs = _v3_philosophy_gate()
        self.assertTrue(passed)
        self.assertGreater(len(msgs), 0)

    def test_v3_gate_asi_ns_locked(self):
        """NS 92.91% LOCKED, V1299 不刷."""
        _, msgs = _v3_philosophy_gate()
        joined = "\n".join(msgs)
        self.assertIn("NS 92.91%", joined)
        self.assertIn("unchanged", joined)
        self.assertIn("not_pretending", joined)


# ============================================================================
# M. Real invariants (3)
# ============================================================================


class TestRealInvariants(unittest.TestCase):
    """真测 invariant (不假装)."""

    def test_real_workspace_channel_is_stable(self):
        """真 workspace channel = stable (实测)."""
        ledger = build_audit_ledger(workspace_root=WORKSPACE)
        self.assertEqual(ledger.channel.value, "stable")
        self.assertTrue(ledger.channel.is_pinned)
        self.assertTrue(ledger.channel.is_known)

    def test_real_workspace_components_has_clippy(self):
        """真 workspace components 含 clippy (CI 必须)."""
        ledger = build_audit_ledger(workspace_root=WORKSPACE)
        self.assertIn("clippy", ledger.components.items)
        self.assertIn("rustfmt", ledger.components.items)
        self.assertEqual(ledger.components.required_missing, [])

    def test_real_workspace_profile_is_minimal(self):
        """真 workspace profile = minimal (rustup accepted)."""
        ledger = build_audit_ledger(workspace_root=WORKSPACE)
        self.assertEqual(ledger.profile.value, "minimal")
        self.assertTrue(ledger.profile.is_valid)


# ============================================================================
# N. V1299 extends V1298 (1)
# ============================================================================


class TestV1299ExtendsV1298(unittest.TestCase):
    """V1299 vs V1298 维度独立 (主 17:43 实事求是)."""

    def test_v1299_dimension_independent_of_v1298(self):
        """V1299 = rust-toolchain.toml, V1298 = Cargo.toml workspace.lints."""
        from apeireth.v1298_cargo_workspace_lints_audit import (
            WORKSPACE_MEMBERS_V1298,
            CARGO_TOML as V1298_CARGO_TOML,
        )
        # 文件名不同 (rust-toolchain.toml vs Cargo.toml)
        self.assertNotEqual(RUST_TOOLCHAIN_TOML, V1298_CARGO_TOML)
        # 维度常量不同
        self.assertNotEqual(len(WORKSPACE_MEMBERS_V1298), 0)  # V1298 维度
        # V1299 components 维度独有
        self.assertIn("clippy", REQUIRED_COMPONENTS)


if __name__ == "__main__":
    unittest.main()
