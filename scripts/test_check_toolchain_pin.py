from __future__ import annotations

import unittest

from scripts.check_toolchain_pin import (
    PINNED_ACTION_COMMITS,
    REF_PATTERN,
    toolchain_ref_problem,
)


CHANNEL = "1.97.1"
OFFICIAL_SHA = "766a42937d994bd78bb4591385445c188ccb3f09"


class ToolchainActionPinTests(unittest.TestCase):
    def test_official_commit_matches_workspace_channel(self):
        self.assertEqual(PINNED_ACTION_COMMITS[CHANNEL], OFFICIAL_SHA)
        self.assertIsNone(toolchain_ref_problem(OFFICIAL_SHA, CHANNEL))

    def test_original_version_ref_remains_valid_for_unmigrated_workflows(self):
        self.assertIsNone(toolchain_ref_problem(CHANNEL, CHANNEL))
        self.assertIsNone(toolchain_ref_problem("nightly", CHANNEL))

    def test_wrong_commit_is_rejected_even_with_truthy_version_comment(self):
        text = f"uses: dtolnay/rust-toolchain@{'0' * 40} # {CHANNEL}"
        ref = REF_PATTERN.search(text).group(1)  # type: ignore[union-attr]
        problem = toolchain_ref_problem(ref, CHANNEL)
        self.assertIsNotNone(problem)
        self.assertIn("官方登记 SHA", problem)

    def test_wrong_mapping_does_not_accept_official_commit(self):
        wrong_mapping = {CHANNEL: "f" * 40}
        problem = toolchain_ref_problem(OFFICIAL_SHA, CHANNEL, wrong_mapping)
        self.assertIsNotNone(problem)
        self.assertIn("不一致", problem)

    def test_commit_for_unregistered_channel_is_rejected(self):
        problem = toolchain_ref_problem(OFFICIAL_SHA, "1.98.1")
        self.assertIsNotNone(problem)
        self.assertIn("没有对应", problem)

    def test_wrong_version_and_floating_aliases_still_fail(self):
        self.assertIsNotNone(toolchain_ref_problem("1.96.1", CHANNEL))
        self.assertIsNotNone(toolchain_ref_problem("stable", CHANNEL))
        self.assertIsNotNone(toolchain_ref_problem("beta", CHANNEL))


if __name__ == "__main__":
    unittest.main()
