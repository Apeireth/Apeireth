"""`scripts/verify-release-assets.ps1`（发布资产校验器）的**行为契约自测**。

# 为什么必须有它
`docs/status/claims.yaml` 的 `rel.release_asset_verify` 原本的 `verify_cmd` 是

    pwsh -NoProfile -File scripts/verify-release-assets.ps1 -ExpectedSha256 <hex> -Path <asset>

⇒ ⇒ 含 `<hex>` / `<asset>` **占位符** ⇒ ⇒ **那条命令根本跑不了**
⇒ ⇒ ⇒ 验证面是**装饰品**（`scripts/check_claims_verify.py` 只能给它挂豁免）
⇒ ⇒ ⇒ 声明升不了级。
⇒ ⇒ ⭐ 本文件把校验器的**核心语义**变成**可自动运行**的窄验证面：

    python3 scripts/test_verify_release_assets.py

⇒ ⇒ 声明的 `verify_cmd` 就可以指向这条**真能跑**的命令（无占位符）。

# 钉住哪些行为（B8 登记的手工 7 用例 ⇒ 此处**改为自动**）
| B8 手工用例 | 本文件的自动用例 |
|---|---|
| 1 正确清单（SHA256SUMS） | `test_matching_manifest_passes` |
| 2 哈希不一致 | `test_mismatching_expected_sha256_fails_truthfully`（**核心语义**） |
| 3 清单缺项 | `test_manifest_missing_entry_fails` |
| 4 `.sha256` sidecar 自动查找 | `test_sidecar_is_discovered` |
| 5 `-ExpectedSha256` 直给 | `test_expected_sha256_direct_passes` |
| 6 `-RequireSignature` | `test_require_signature_gate_rejects_unsigned` |
| 7 参数不合法（短哈希） | `test_invalid_expected_sha256_is_rejected` |

外加两条**边界行为**（fail-closed，不得「猜」）：
资产文件不存在 ⇒ 失败；`-ExpectedSha256` 与 `-ManifestPath` 叠加 ⇒ 失败。

# ⚠️ 诚实边界（本自测**不**证明什么 —— 别拿它当签名/来源的证据）
- ⭐ **「签名状态如实报」≠「签名有效」**：本文件只断言脚本**如实转述**平台校验面
  （Windows = 未签名 / 有效；非 Windows = 「本平台无 Authenticode 校验面」），
  **不**断言任何资产**已签名** ⇒ ⇒ 发布物**代码签名**仍属**排期待办**；
- ⭐ **哈希一致 ≠ 二进制来源可证**（可复现构建证明 = 排期待办）⇒ ⇒ 本文件同样不证明它。

# ⭐ 反向验伪（AGENTS.md §5.3）：本文件**不是恒绿断言**
    APEIRETH_VERIFY_TEST_FAULT=wrong-expected-hash python3 scripts/test_verify_release_assets.py

⇒ ⇒ 期望哈希被**故意换错** ⇒ ⇒ 脚本如实报 FAIL ⇒ ⇒ **本文件必须变红**（附完整输出）。
"""

from pathlib import Path
import hashlib
import os
import shutil
import subprocess
import tempfile
import unittest

VERIFIER = Path(__file__).resolve().with_name("verify-release-assets.ps1")

#: 夹具内容 + 其**独立算出**的 SHA-256（硬编码 ⇒ 内容变了必须同步改这里；
#: 另有 `test_fixture_hash_constant_is_not_stale` 用 hashlib 复算 ⇒ 防止常量变成过期谎言）。
FIXTURE_BYTES = b"apeireth-release-asset-fixture\n"
FIXTURE_SHA256 = "f7374833617daaf3360175e90420b6b7a5e4f5e6ba2627fb99cb1e0359e35a77"
FIXTURE_NAME = "apeireth-fixture.bin"

#: B8 手工用例 2 用的「全零期望」⇒ 这里当**故意错**的期望哈希。
WRONG_SHA256 = "0" * 64

#: 反向验伪开关（AGENTS.md §5.3）—— 只会让本文件**变红**，不会让它变绿。
FAULT_ENV = "APEIRETH_VERIFY_TEST_FAULT"
FAULT_WRONG_EXPECTED = "wrong-expected-hash"


class VerifyReleaseAssetsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.artifact = self.root / FIXTURE_NAME
        self.artifact.write_bytes(FIXTURE_BYTES)

    # ---- helpers -------------------------------------------------------- #
    def run_verifier(self, *args: str) -> subprocess.CompletedProcess:
        """跑校验器（真实 pwsh 子进程 ⇒ 断言的是**脚本的真行为**，不是重写的逻辑）。"""
        return subprocess.run(
            ["pwsh", "-NoProfile", "-File", str(VERIFIER), *args],
            cwd=str(self.root),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
        )

    def expected_hash(self) -> str:
        """本用例**期望**的哈希；故障注入开关下**故意**给错（用于反向验伪）。"""
        if os.environ.get(FAULT_ENV) == FAULT_WRONG_EXPECTED:
            return WRONG_SHA256
        return FIXTURE_SHA256

    def write_manifest(self, name: str, entry_name: str, digest: str) -> Path:
        """写一份 SHA256SUMS 风格的清单（`<哈希>  <文件名>`）。"""
        manifest = self.root / name
        manifest.write_text(f"{digest}  {entry_name}\n", encoding="utf-8")
        return manifest

    # ---- 夹具自身的诚实性 ---------------------------------------------- #
    def test_fixture_hash_constant_is_not_stale(self):
        """硬编码的期望哈希必须与夹具内容一致（否则下面的比对是**自说自话**）。"""
        self.assertEqual(hashlib.sha256(FIXTURE_BYTES).hexdigest(), FIXTURE_SHA256)

    # ---- B8 用例 5：`-ExpectedSha256` 直给 ⇒ PASS ----------------------- #
    def test_expected_sha256_direct_passes(self):
        result = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ExpectedSha256", self.expected_hash()
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"[PASS] {FIXTURE_NAME}", result.stdout)
        self.assertIn("RESULT: PASS", result.stdout)
        self.assertIn(FIXTURE_SHA256, result.stdout)

    # ---- B8 用例 1：正确清单 ⇒ PASS ------------------------------------- #
    def test_matching_manifest_passes(self):
        manifest = self.write_manifest("SHA256SUMS", FIXTURE_NAME, self.expected_hash())
        result = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ManifestPath", str(manifest)
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"[PASS] {FIXTURE_NAME}", result.stdout)
        self.assertIn("RESULT: PASS", result.stdout)
        # PASS 行必须**自带边界句**（不得把「哈希一致」写成「来源已证明」）。
        self.assertIn("可复现构建证明 = 排期待办", result.stdout)

    # ---- B8 用例 4：同目录 `.sha256` sidecar 自动查找 ⇒ PASS ------------- #
    def test_sidecar_is_discovered(self):
        sidecar = Path(f"{self.artifact}.sha256")
        sidecar.write_text(f"{self.expected_hash()}  {FIXTURE_NAME}\n", encoding="utf-8")
        result = self.run_verifier("-ArtifactPath", str(self.artifact))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("RESULT: PASS", result.stdout)
        self.assertIn(f"期望（sidecar {sidecar.name}）", result.stdout)

    # ---- ⭐ 核心语义：期望 SHA-256 与实际不符 ⇒ 必须如实报 FAIL -------- #
    def test_mismatching_expected_sha256_fails_truthfully(self):
        result = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ExpectedSha256", WRONG_SHA256
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"[FAIL] {FIXTURE_NAME}", result.stdout)
        # 如实外显：必须点名「哈希不一致」并给出「不要安装」。
        self.assertIn("哈希不一致", result.stdout)
        self.assertIn("不要安装", result.stdout)
        # ⭐ 不得粉饰：结论行必须是 FAIL，且**不许**出现 PASS。
        self.assertIn("RESULT: FAIL", result.stdout)
        self.assertNotIn("RESULT: PASS", result.stdout)
        # 两个哈希都要如实打出来（人可自行核对）。
        self.assertIn(WRONG_SHA256, result.stdout)
        self.assertIn(FIXTURE_SHA256, result.stdout)

    # ---- B8 用例 3：清单缺项 ⇒ FAIL（缺项不装过） ---------------------- #
    def test_manifest_missing_entry_fails(self):
        manifest = self.write_manifest("SHA256SUMS", "some-other-artifact.bin", FIXTURE_SHA256)
        result = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ManifestPath", str(manifest)
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"[FAIL] {FIXTURE_NAME}", result.stdout)
        self.assertIn("缺项不装过", result.stdout)
        self.assertIn("RESULT: FAIL", result.stdout)
        self.assertNotIn("RESULT: PASS", result.stdout)

    # ---- B8 用例 6：`-RequireSignature` 门槛 ⇒ 非有效即失败 ------------- #
    def test_require_signature_gate_rejects_unsigned(self):
        """⚠️ 只断言**门槛语义**（非「有效」即失败）+ **如实报**状态。

        ⚠️ **不**断言「资产已签名」—— 本夹具**没有**签名，
        且在非 Windows 平台连 Authenticode 校验面都不存在（脚本如实报「未验证」）。
        """
        result = self.run_verifier(
            "-ArtifactPath",
            str(self.artifact),
            "-ExpectedSha256",
            FIXTURE_SHA256,
            "-RequireSignature",
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("RESULT: FAIL", result.stdout)
        self.assertNotIn("RESULT: PASS", result.stdout)
        # 签名状态必须**如实转述**（有签名行），且**不许**谎称「签名有效」。
        self.assertIn("签名:", result.stdout)
        self.assertNotIn("签名有效", result.stdout)
        # 默认门槛下未签名**不**判失败 ⇒ ⇒ 签名状态只如实报告、不参与成败。
        lenient = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ExpectedSha256", FIXTURE_SHA256
        )
        self.assertEqual(lenient.returncode, 0, lenient.stdout + lenient.stderr)
        self.assertIn("RESULT: PASS", lenient.stdout)
        self.assertIn("签名:", lenient.stdout)
        self.assertNotIn("签名有效", lenient.stdout)

    # ---- B8 用例 7：参数不合法 ⇒ 拒绝不猜 ----------------------------- #
    def test_invalid_expected_sha256_is_rejected(self):
        result = self.run_verifier(
            "-ArtifactPath", str(self.artifact), "-ExpectedSha256", "abc123"
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("不是 64 位十六进制哈希", result.stdout)
        self.assertNotIn("RESULT: PASS", result.stdout)

    # ---- 边界：两个哈希口径叠加 ⇒ 拒绝（避免双口径） ------------------- #
    def test_expected_sha256_and_manifest_are_mutually_exclusive(self):
        manifest = self.write_manifest("SHA256SUMS", FIXTURE_NAME, FIXTURE_SHA256)
        result = self.run_verifier(
            "-ArtifactPath",
            str(self.artifact),
            "-ExpectedSha256",
            FIXTURE_SHA256,
            "-ManifestPath",
            str(manifest),
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("二选一", result.stdout)

    # ---- 边界：资产不存在 ⇒ fail-closed -------------------------------- #
    def test_missing_asset_fails_closed(self):
        result = self.run_verifier(
            "-ArtifactPath", str(self.root / "does-not-exist.bin"),
            "-ExpectedSha256", FIXTURE_SHA256,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("资产文件不存在", result.stdout)
        self.assertIn("RESULT: FAIL", result.stdout)
        self.assertNotIn("RESULT: PASS", result.stdout)


if __name__ == "__main__":
    if not shutil.which("pwsh"):
        # ⚠️ **不许静默跳过**（跳过 = 恒绿 = 装饰品）⇒ ⇒ 缺 pwsh 就**直接失败**。
        raise SystemExit("pwsh is required to verify the release asset verifier")
    unittest.main()
