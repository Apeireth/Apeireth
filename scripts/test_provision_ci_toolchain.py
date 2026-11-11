#!/usr/bin/env python3
"""CI toolchain behavior tests; no downloads or shared toolchain mutations."""

from __future__ import annotations

import contextlib
import io
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import provision_ci_toolchain as provisioner


class ProvisionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="apeireth-toolchain-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runner_temp = self.root / "runner-temp"
        self.runner_temp.mkdir()
        self.output = self.root / "github-env"
        self.spec("1.97.1", ["rustfmt", "clippy", "rust-src"])
        self.env = {
            "GITHUB_ACTIONS": "true", "GITHUB_ENV": str(self.output),
            "RUNNER_TEMP": str(self.runner_temp), "CARGO_HOME": "existing-proxies",
            "RUSTUP_HOME": "shared-rustup", "RUSTUP_TOOLCHAIN": "wrong-inherited-pin",
        }
        self.calls = []
        self.compiler = "1.97.1"
        self.components = ["rustfmt-x86_64-unknown-linux-gnu", "clippy-x86_64-unknown-linux-gnu", "rust-src"]

    def spec(self, channel, components):
        (self.root / "rust-toolchain.toml").write_text(
            f'[toolchain]\nchannel = "{channel}"\nprofile = "minimal"\n'
            + "components = [" + ", ".join(f'"{c}"' for c in components) + "]\n",
            encoding="utf-8",
        )

    def fake_run(self, command, **kwargs):
        self.calls.append((command, kwargs))
        output = ""
        if command[1:3] == ["component", "list"]:
            output = "\n".join(self.components)
        elif command[-2:] == ["rustc", "--version"]:
            output = f"rustc {self.compiler} (test-build)\n"
        return subprocess.CompletedProcess(command, 0, stdout=output)

    def invoke(self, env=None, run=None):
        with contextlib.redirect_stdout(io.StringIO()):
            return provisioner.provision(self.root, env or self.env, run or self.fake_run)

    def test_fresh_home_pin_components_and_existing_proxies(self):
        previous = self.runner_temp / "apeireth-merge-gate-rustup-partial"
        previous.mkdir()
        marker = previous / "partial-component"
        marker.write_text("keep", encoding="utf-8")
        home = self.invoke()
        second_home = self.invoke()
        self.assertNotEqual(home, second_home)
        self.assertNotEqual(home, previous)
        self.assertTrue(home.is_relative_to(self.runner_temp))
        self.assertEqual(marker.read_text(encoding="utf-8"), "keep")
        command, kwargs = self.calls[0]
        self.assertEqual(command, [
            "rustup", "toolchain", "install", "1.97.1", "--profile", "minimal",
            "--no-self-update", "--component", "rustfmt", "--component", "clippy",
            "--component", "rust-src",
        ])
        self.assertEqual(kwargs["env"]["CARGO_HOME"], "existing-proxies")
        self.assertEqual(kwargs["env"]["RUSTUP_HOME"], str(home))
        self.assertEqual(kwargs["env"]["RUSTUP_TOOLCHAIN"], "1.97.1")
        self.assertIn(f"RUSTUP_HOME={home}\nRUSTUP_TOOLCHAIN=1.97.1\n", self.output.read_text(encoding="utf-8"))

    def test_requires_ci_environment_before_running_commands(self):
        for key in ("GITHUB_ACTIONS", "GITHUB_ENV", "RUNNER_TEMP"):
            with self.subTest(key=key):
                env = dict(self.env)
                del env[key]
                with self.assertRaisesRegex(ValueError, "CI only"):
                    self.invoke(env)
        self.assertEqual(self.calls, [])

    def test_rejects_home_outside_runner_temp(self):
        with patch.object(provisioner.tempfile, "mkdtemp", return_value=str(self.root)):
            with self.assertRaisesRegex(ValueError, "strictly inside"):
                self.invoke()
        self.assertEqual(self.calls, [])

    def test_follows_changed_single_source(self):
        self.spec("1.98.2", ["rustfmt", "rust-src"])
        self.compiler = "1.98.2"
        self.invoke()
        command = self.calls[0][0]
        self.assertEqual(command[3], "1.98.2")
        self.assertNotIn("clippy", command)
        self.assertTrue(self.output.read_text(encoding="utf-8").endswith("RUSTUP_TOOLCHAIN=1.98.2\n"))

    def test_rejects_drifted_installed_compiler(self):
        self.compiler = "1.96.0"
        with self.assertRaisesRegex(ValueError, "does not match"):
            self.invoke()
        self.assertFalse(self.output.exists())

    def test_rejects_missing_declared_component(self):
        self.components.remove("rust-src")
        with self.assertRaisesRegex(ValueError, "not installed: rust-src"):
            self.invoke()
        self.assertFalse(self.output.exists())

    def test_install_failure_is_not_exported(self):
        def fail(command, **kwargs):
            raise subprocess.CalledProcessError(1, command)
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke(run=fail)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
