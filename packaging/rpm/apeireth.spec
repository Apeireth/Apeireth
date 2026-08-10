# Apeireth rpm spec (8 包之 1, D-06 拍板, Linux 重点优化)
# 平台: RHEL / Fedora / CentOS (dnf install apeireth)
# 工具: cargo-rpm
# 体积: ~50MB (含 systemd unit + config)
#
# 用法:
#   cargo install cargo-rpm
#   ./packaging/rpm/build.sh   # 出 target/rpm/apeireth-1.0.0-1.x86_64.rpm
#
# 验证:
#   sudo dnf install ./target/rpm/apeireth-1.0.0-1.x86_64.rpm
#   sudo systemctl start apeireth
#   curl http://localhost:8080/health
#
# 卸载: sudo dnf remove apeireth

Name:           apeireth
Version:        1.0.0
Release:        1%{?dist}
Summary:        Apeireth OS - AI Growth Platform (API server)
License:        Apache-2.0
URL:            https://github.com/apeireth/apeireth-rust
Source0:        https://github.com/apeireth/apeireth-rust/archive/refs/tags/v%{version}.tar.gz

# Build 依赖 (开发机)
BuildRequires:  cargo >= 1.80
BuildRequires:  rust >= 1.80
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(sqlite3)
BuildRequires:  pkgconfig(libgit2)
BuildRequires:  systemd

# 运行时依赖 (用户机)
Requires:       openssl-libs >= 3.0
Requires:       sqlite-libs >= 3.40
Requires:       libgit2 >= 1.7
Requires:       ca-certificates

%description
Apeireth OS — 长程 AI 成长平台 (立体架构 v2 + 生命架构 v4).
本包提供 API server (apeireth) 二进制, 默认监听 :8080 (HTTP/WS) + :9090 (metrics).
配套 PostgreSQL 16 + Redis 7 见 docker-compose.yml 或 packaging/docker/.

%prep
%autosetup -n apeireth-rust-%{version}

%build
cargo build --release --bin apeireth --locked
strip target/release/apeireth

%install
install -Dm755 target/release/apeireth %{buildroot}%{_bindir}/apeireth
install -Dm644 packaging/deb/apeireth.service %{buildroot}%{_unitdir}/apeireth.service

# 数据 / 配置 / 日志目录
install -dm750 %{buildroot}%{_sharedstatedir}/apeireth
install -dm750 %{buildroot}%{_sysconfdir}/apeireth
install -dm755 %{buildroot}%{_localstatedir}/log/apeireth

%pre
getent group apeireth >/dev/null || groupadd --system apeireth
getent passwd apeireth >/dev/null || useradd --system \
    --gid apeireth --home-dir %{_sharedstatedir}/apeireth \
    --shell /sbin/nologin --comment "Apeireth OS daemon" apeireth

%post
%systemd_postun_with_restart apeireth.service || :

%preun
%systemd_preun apeireth.service || :

%postun
%systemd_postun_with_restart apeireth.service || :

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/apeireth
%{_unitdir}/apeireth.service
%dir %attr(750, apeireth, apeireth) %{_sharedstatedir}/apeireth
%dir %attr(750, apeireth, apeireth) %{_localstatedir}/log/apeireth
%ghost %config(noreplace) %{_sysconfdir}/apeireth/

%changelog
* Wed Aug 05 2026 Apeireth Team <dev@apeireth.io> - 1.0.0-1
- R20 阶段 3 首次 1.0 release (8 包齐发, D-06 拍板)
- API server (apeireth) + systemd unit
