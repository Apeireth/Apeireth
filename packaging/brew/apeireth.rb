# Apeireth Homebrew Formula (8 包之 1, D-06 拍板)
# 平台: macOS (brew install apeireth/tap/apeireth)
# 体积: ~40MB (含 launchd plist)
# 验证: brew install apeireth/tap/apeireth
#        brew services start apeireth
#        curl http://localhost:8080/health
# 卸载: brew uninstall apeireth

class Apeireth < Formula
  desc "Apeireth OS - AI Growth Platform (API server)"
  homepage "https://github.com/apeireth/apeireth-rust"
  url "https://github.com/apeireth/apeireth-rust/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "REPLACE_WITH_RELEASE_SHA256_AT_TAG_TIME"
  license "Apache-2.0"
  version "1.0.0"

  # macOS 版本兼容
  depends_on :macos => :high_sierra

  # 编译依赖
  depends_on "rust" => :build
  depends_on "pkg-config" => :build
  depends_on "openssl@3" => :build
  depends_on "sqlite" => :build
  depends_on "libgit2" => :build

  # 运行时依赖 (bottle 不需要)
  depends_on "openssl@3"
  depends_on "sqlite"
  depends_on "libgit2"

  def install
    # 编译 (musl 静态链接不可用 — macOS 走 system openssl)
    system "cargo", "build", "--release", "--bin", "apeireth", "--locked"
    bin.install "target/release/apeireth"
  end

  # 启动后 sanity check
  test do
    system "#{bin}/apeireth", "--version"
  end

  # launchd plist (per blueprint §3.4 提到 launchd 集成)
  service do
    run [opt_bin/"apeireth", "serve"]
    keep_alive true
    log_path var/"log/apeireth.log"
    error_log_path var/"log/apeireth.err"
    working_dir HOMEBREW_PREFIX
  end
end
