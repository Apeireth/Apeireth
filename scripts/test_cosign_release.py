"""Fail-closed contracts for the release signing shell scripts."""

from pathlib import Path
import base64
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SIGN = ROOT / "scripts" / "release" / "cosign-sign-all.sh"
VERIFY = ROOT / "scripts" / "release" / "cosign-verify.sh"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release-1.0.0.yml"
FIVE_PACKAGE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
COSIGN_WORKFLOW = ROOT / ".github" / "workflows" / "cosign.yml"
COSIGN_INSTALLER_SHA = "756fe2d284c7292c4e9b62c8a28f1e5629c59532"
SCOOP_BUILD = ROOT / "packaging" / "scoop" / "build.ps1"
SCOOP_MANIFEST = ROOT / "packaging" / "scoop" / "apeireth.json"
VERSION = "1.0.0"
EXPECTED = [
    f"apeireth_{VERSION}_amd64.deb",
    f"apeireth-{VERSION}-1.x86_64.rpm",
    "apeireth.rb",
    "apeireth.json",
    f"apeireth-{VERSION}-linux-x86_64.tar.gz",
    f"apeireth-{VERSION}-windows-x86_64.zip",
    f"apeireth-{VERSION}-windows-x86_64.msi",
]


@unittest.skipUnless(shutil.which("bash"), "bash is required")
class CosignReleaseContracts(unittest.TestCase):
    def run_sign(self, dist: Path):
        env = {**os.environ, "COSIGN_BIN": "true", "DIST_DIR": str(dist)}
        return subprocess.run(
            ["bash", str(SIGN), "--dry-run"],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )

    def test_missing_expected_artifact_fails_even_in_dry_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.run_sign(Path(tmp))
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_complete_expected_manifest_passes_dry_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            for name in EXPECTED:
                (dist / name).write_bytes(b"synthetic release fixture\n")
            result = self.run_sign(dist)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_blob_verification_requires_sigstore_bundle(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package = root / "package.zip"
            package.write_bytes(b"package")
            public_key = root / "cosign.pub"
            public_key.write_text("synthetic public key")
            env = {
                **os.environ,
                "COSIGN_BIN": "true",
                "COSIGN_PUB": str(public_key),
            }
            result = subprocess.run(
                ["bash", str(VERIFY), str(package)],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=30,
            )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bundle", result.stdout + result.stderr)

    def test_live_sign_decodes_key_without_logging_and_cleans_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = root / "dist"
            dist.mkdir()
            for name in EXPECTED:
                (dist / name).write_bytes(b"synthetic release fixture\n")

            public_key = root / "cosign.pub"
            public_key.write_text("synthetic reviewed public key\n")
            key_record = root / "key-path.txt"
            fake_cosign = root / "cosign"
            fake_cosign.write_text(
                """#!/usr/bin/env bash
set -euo pipefail
command_name="${1:-}"
shift || true
case "${command_name}" in
  version)
    echo "cosign synthetic"
    ;;
  public-key)
    key_path=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --key) key_path="$2"; shift 2 ;;
        *) shift ;;
      esac
    done
    test -s "${key_path}"
    test "$(stat -c '%a' "${key_path}")" = "600"
    test "$(sha256sum "${key_path}" | awk '{print $1}')" = "${FAKE_KEY_SHA256}"
    printf '%s\\n' "${key_path}" > "${FAKE_KEY_RECORD}"
    cat "${FAKE_PUBLIC_KEY}"
    ;;
  sign-blob)
    bundle=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --yes) shift ;;
        --key) test -s "$2"; shift 2 ;;
        --bundle) bundle="$2"; shift 2 ;;
        *) shift ;;
      esac
    done
    printf 'bundle\\n' > "${bundle}"
    ;;
  verify-blob)
    exit 0
    ;;
  *)
    exit 2
    ;;
esac
"""
            )
            fake_cosign.chmod(0o755)

            private_key = b"synthetic-private-key-material-never-log"
            encoded_key = base64.b64encode(private_key).decode("ascii")
            env = {
                **os.environ,
                "COSIGN_BIN": str(fake_cosign),
                "COSIGN_KEY_B64": encoded_key,
                "COSIGN_PASSWORD": "synthetic-password-never-log",
                "COSIGN_PUB": str(public_key),
                "DIST_DIR": str(dist),
                "FAKE_KEY_SHA256": hashlib.sha256(private_key).hexdigest(),
                "FAKE_KEY_RECORD": str(key_record),
                "FAKE_PUBLIC_KEY": str(public_key),
            }
            result = subprocess.run(
                ["bash", str(SIGN)], cwd=ROOT, env=env,
                capture_output=True, text=True, encoding="utf-8",
                errors="replace", timeout=30,
            )
            output = result.stdout + result.stderr
            self.assertEqual(result.returncode, 0, output)
            self.assertNotIn(private_key.decode("ascii"), output)
            self.assertNotIn(encoded_key, output)
            self.assertNotIn(env["COSIGN_PASSWORD"], output)
            decoded_path = Path(key_record.read_text().strip())
            self.assertFalse(decoded_path.exists(), "temporary private key survived signing")

    def test_release_workflow_produces_the_strict_signing_manifest(self):
        release = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        five_package_release = FIVE_PACKAGE_WORKFLOW.read_text(encoding="utf-8")
        cosign = COSIGN_WORKFLOW.read_text(encoding="utf-8")
        self.assertLess(
            release.index('"packaging/zip/build.ps1"'),
            release.index('"packaging/scoop/build.ps1"'),
        )
        for path in [
            "target/*.tar.gz",
            "target/*.zip",
            "apeireth-${VERSION}-linux-x86_64.tar.gz",
            "apeireth-${VERSION}-windows-x86_64.zip",
            "apeireth-${VERSION}-windows-x86_64.msi",
        ]:
            with self.subTest(path=path):
                self.assertIn(path, release + cosign)
        self.assertNotIn("darwin-universal.tar.gz", cosign)
        self.assertLess(
            five_package_release.index("packaging/zip/build.ps1"),
            five_package_release.index("packaging/scoop/build.ps1"),
        )
        self.assertIn("$parsed.architecture.'64bit'", five_package_release)
        self.assertIn("Get-FileHash -LiteralPath $zip", five_package_release)

    def test_cosign_toolchain_and_bundle_verification_are_pinned(self):
        workflow = COSIGN_WORKFLOW.read_text(encoding="utf-8")
        sign_script = SIGN.read_text(encoding="utf-8")
        verify_script = VERIFY.read_text(encoding="utf-8")
        self.assertEqual(workflow.count(f"cosign-installer@{COSIGN_INSTALLER_SHA}"), 4)
        self.assertEqual(workflow.count("cosign-release: 'v3.1.4'"), 4)
        self.assertNotIn("cosign-installer@v", workflow)
        for text in (workflow, sign_script, verify_script):
            with self.subTest(source=text[:40]):
                self.assertIn("verify-blob", text)
                self.assertIn("--bundle", text)
                self.assertIn(".sigstore.json", text)

    def test_manual_dispatch_does_not_generate_a_key_by_default(self):
        workflow = COSIGN_WORKFLOW.read_text(encoding="utf-8")
        self.assertRegex(
            workflow,
            r"run_keygen:\s*\n(?:\s+.*\n)*?\s+default: false",
        )
        self.assertIn(
            "if: github.event_name == 'workflow_dispatch' && inputs.run_keygen == true",
            workflow,
        )

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is required")
    def test_scoop_manifest_binds_to_the_locally_built_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            scoop_dir = root / "packaging" / "scoop"
            scoop_dir.mkdir(parents=True)
            shutil.copyfile(SCOOP_BUILD, scoop_dir / "build.ps1")
            shutil.copyfile(SCOOP_MANIFEST, scoop_dir / "apeireth.json")
            target = root / "target"
            target.mkdir()
            zip_path = target / f"apeireth-{VERSION}-windows-x86_64.zip"
            zip_bytes = b"synthetic canonical Windows ZIP\n"
            zip_path.write_bytes(zip_bytes)
            repository = "Apeireth/Apeireth"
            env = {
                **os.environ,
                "APEIRETH_VERSION": VERSION,
                "GITHUB_REPOSITORY": repository,
            }
            result = subprocess.run(
                ["pwsh", "-NoProfile", "-File", str(scoop_dir / "build.ps1")],
                cwd=root, env=env, capture_output=True, text=True,
                encoding="utf-8", errors="replace", timeout=30,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            manifest = json.loads((scoop_dir / "apeireth.json").read_text())
            release = manifest["architecture"]["64bit"]
            self.assertEqual(manifest["version"], VERSION)
            self.assertEqual(
                release["url"],
                f"https://github.com/{repository}/releases/download/"
                f"v{VERSION}/apeireth-{VERSION}-windows-x86_64.zip",
            )
            self.assertEqual(release["hash"].lower(), hashlib.sha256(zip_bytes).hexdigest())
            self.assertEqual(manifest["homepage"], f"https://github.com/{repository}")
            self.assertEqual(manifest["checkver"]["github"], f"https://github.com/{repository}")
            self.assertEqual(
                manifest["autoupdate"]["architecture"]["64bit"]["url"],
                f"https://github.com/{repository}/releases/download/"
                "v$version/apeireth-$version-windows-x86_64.zip",
            )

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is required")
    def test_scoop_manifest_rejects_invalid_repository_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            scoop_dir = root / "packaging" / "scoop"
            scoop_dir.mkdir(parents=True)
            shutil.copyfile(SCOOP_BUILD, scoop_dir / "build.ps1")
            shutil.copyfile(SCOOP_MANIFEST, scoop_dir / "apeireth.json")
            target = root / "target"
            target.mkdir()
            (target / f"apeireth-{VERSION}-windows-x86_64.zip").write_bytes(b"zip")
            env = {
                **os.environ,
                "APEIRETH_VERSION": VERSION,
                "GITHUB_REPOSITORY": "https://invalid.example/repository",
            }
            result = subprocess.run(
                ["pwsh", "-NoProfile", "-File", str(scoop_dir / "build.ps1")],
                cwd=root, env=env, capture_output=True, text=True,
                encoding="utf-8", errors="replace", timeout=30,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
