# Apeireth Packaging Suite (APEIRETH 2.0 RC)

This directory contains packaging specifications, manifests, and build automation scripts for all supported Apeireth distribution targets.

## Directory Layout

```text
packaging/
├── zip/                  # Windows CLI Portable ZIP
│   ├── build.ps1         # Builds apeireth-2.0.0-rc.1-windows-x86_64.zip
│   ├── build-zip.ps1     # Alias wrapper
│   ├── install.ps1       # User installation helper
│   └── uninstall.ps1     # User uninstallation helper
├── msi/                  # Windows WiX MSI Installer
│   ├── apeireth.wxs      # WiX product XML definition (valid GUIDs, Start Menu, clean uninstall)
│   ├── build.ps1         # WiX build & staging script
│   ├── install-msi.ps1   # MSI installation helper
│   ├── uninstall-msi.ps1 # MSI uninstallation helper
│   ├── Cargo.toml.snippet# cargo-wix metadata snippet
│   └── icon.ico          # Application icon
├── scoop/                # Scoop Package Manager
│   ├── apeireth.json     # Scoop manifest definition
│   ├── build.ps1         # Scoop manifest staging script
│   ├── build-scoop.ps1   # Alias wrapper
│   ├── install-scoop.ps1 # User installation helper
│   └── uninstall-scoop.ps1# Clean uninstaller with 0-residue checks
├── desktop/              # Tauri Desktop Suite (MSI + NSIS + Portable)
│   ├── stage-desktop.ps1 # Stages Svelte 5 frontend, icons, binaries
│   ├── build-desktop-msi.ps1 # Builds Tauri Desktop MSI bundle
│   ├── build-desktop-nsis.ps1# Builds Tauri Desktop NSIS installer
│   └── build-desktop.ps1 # Master desktop packaging orchestrator
├── test-packaging.ps1    # Automated packaging audit and validation test
├── deb/                  # Debian / Ubuntu package
├── rpm/                  # RHEL / Fedora / CentOS package
├── tarball/              # Linux generic musl static archive
├── brew/                 # macOS Homebrew tap formula
└── docker/               # Container image definition
```

## Quick Build Commands (Windows)

```powershell
# Build All Windows Packages
.\scripts\build-all-packages.ps1

# Build CLI Portable ZIP only
.\packaging\zip\build.ps1

# Build CLI MSI Installer only
.\packaging\msi\build.ps1

# Build Desktop Suite (Portable + MSI + NSIS)
.\packaging\desktop\build-desktop.ps1

# Run Packaging Test Suite
.\packaging\test-packaging.ps1
```

For complete lifecycle details (install, upgrade, repair, uninstall), see [docs/packaging/windows-packaging-lifecycle.md](../docs/packaging/windows-packaging-lifecycle.md).
