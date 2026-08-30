# Apeireth rpm spec (Apeireth 2.0 RC1, Linux packaging)
# 平台: RHEL 8+/9+ / Fedora / CentOS Stream / Rocky / Alma (dnf install apeireth)
# 工具: cargo-rpm / rpmbuild
# 体积: ~45MB (含 systemd unit + config)
#
# 用法:
#   ./packaging/rpm/build.sh   # 出 target/rpm/apeireth-2.0.0-0.1.rc1.*.rpm
#
# 验证:
#   sudo dnf install ./target/rpm/apeireth-2.0.0-0.1.rc1.*.rpm
#   sudo systemctl start apeireth
#   curl http://localhost:8080/health
#
# 卸载: sudo dnf remove apeireth

Name:           apeireth
Version:        2.0.0
Release:        0.1.rc1%{?dist}
Summary:        Apeireth 2.0 - AGI Operating System & Cognitive Microkernel
License:        Apache-2.0
URL:            https://github.com/apeireth/apeireth-rust
Source0:        https://github.com/apeireth/apeireth-rust/archive/refs/tags/v%{version}-rc.1.tar.gz

# Build 依赖 (开发机 / CI runner)
BuildRequires:  cargo >= 1.80
BuildRequires:  rust >= 1.80
BuildRequires:  ca-certificates
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  systemd

# 运行时依赖 (用户机)
Requires:       ca-certificates
Requires:       systemd

%description
Apeireth OS 2.0 — Cognitive Microkernel & AGI Operating System.
本包提供 Apeireth CLI / Gateway 二进制 (apeireth), 默认网关监听 :8080 (HTTP/WS).
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
* Sun Aug 30 2026 Apeireth Team <dev@apeireth.io> - 2.0.0-0.1.rc1
- APEIRETH 2.0 RC1 release (8 packages, unified workspace)
- Cognitive Microkernel & Canonical Agent Loop + Gateway Serve (:8080)
- Keyring Secret Service + Physical Sandbox Integration

* Wed Aug 05 2026 Apeireth Team <dev@apeireth.io> - 1.0.0-1
- R20 阶段 3 首次 1.0 release (8 包齐发, D-06 拍板)
- API server (apeireth) + systemd unit
