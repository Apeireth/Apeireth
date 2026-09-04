# Apeireth 2.0 RC — Windows Packaging & Lifecycle Specification

## Overview

Apeireth 2.0 RC provides multiple distribution formats for Windows environments:
1. **CLI Portable ZIP** (`apeireth-2.0.0-rc.1-windows-x86_64.zip`): Standalone portable archive containing `apeireth.exe`, `LICENSE`, `NOTICE`, `README.md`, launcher scripts, and configuration templates.
2. **WiX Windows MSI Installer** (`apeireth-2.0.0-rc.1-windows-x86_64.msi`): System-wide enterprise installer with auto-generated `ProductCode`, stable `UpgradeCode`, Start Menu shortcuts, icon integration, PATH configuration, Windows Service registration, and clean uninstall.
3. **Scoop Manifest** (`packaging/scoop/apeireth.json`): Package manager distribution for seamless CLI installation and updates.
4. **Tauri Desktop Suite** (MSI & NSIS installers): Desktop companion application packaging supporting both perMachine system installs and clean desktop/Start Menu shortcut integration.

---

## 1. Distribution Matrix

| Package Type | Artifact Pattern | Install Target | Service / Background Support | Lifecycle Management |
| :--- | :--- | :--- | :--- | :--- |
| **Portable ZIP** | `apeireth-2.0.0-rc.1-windows-x86_64.zip` | User-selected / `%LOCALAPPDATA%\Programs\Apeireth` | Optional via NSSM | `packaging/zip/install.ps1`, `uninstall.ps1` |
| **WiX MSI** | `apeireth-2.0.0-rc.1-windows-x86_64.msi` | `%ProgramFiles%\Apeireth\` | Native Windows Service (`ApeirethOS`) | `msiexec /i`, `msiexec /x`, `msiexec /f` |
| **Scoop** | `packaging/scoop/apeireth.json` | `~/scoop/apps/apeireth/current` | Direct CLI / Background serve | `scoop install`, `scoop update`, `scoop uninstall` |
| **Desktop MSI** | `apeireth-companion-2.0.0-rc.1.msi` | `%ProgramFiles%\Apeireth Companion\` | GUI Tray / Autostart plugin | Standard Windows MSI lifecycle |
| **Desktop NSIS**| `apeireth-companion-2.0.0-rc.1-setup.exe` | `%ProgramFiles%\Apeireth Companion\` | GUI Tray / Autostart plugin | Standard NSIS installer/uninstaller |

---

## 2. CLI Portable ZIP Lifecycle

### Package Structure
```text
apeireth-2.0.0-rc.1-windows-x86_64.zip
├── apeireth.exe                  # Primary CLI executable (root level)
├── LICENSE                       # Apache License 2.0
├── NOTICE                        # Third-party attribution & notices
├── README.md                     # Documentation
├── README.txt                    # Quickstart guide
├── bin/
│   ├── apeireth.exe              # Mirror binary for bin/ PATH conventions
│   └── apeireth-serve.bat        # Windows serve launcher batch script
├── config/
│   └── apeireth.env.example      # Environment variables template
└── share/
    ├── LICENSE
    ├── NOTICE
    └── README.md
```

### Lifecycle Operations

#### Installation
1. Extract ZIP to destination:
   ```powershell
   Expand-Archive -Path target\apeireth-2.0.0-rc.1-windows-x86_64.zip -DestinationPath "$env:LOCALAPPDATA\Programs\Apeireth" -Force
   ```
2. Or use helper script:
   ```powershell
   .\packaging\zip\install.ps1
   ```
3. Add directory to User `PATH`:
   ```powershell
   $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
   [Environment]::SetEnvironmentVariable("Path", "$userPath;$env:LOCALAPPDATA\Programs\Apeireth;$env:LOCALAPPDATA\Programs\Apeireth\bin", "User")
   ```

#### Verification
```powershell
apeireth --version
apeireth serve
```

#### Upgrade
- Overwrite existing directory with new archive contents. User data in `$env:USERPROFILE\.apeireth` remains intact.

#### Uninstallation
```powershell
.\packaging\zip\uninstall.ps1
```
- Stops running processes, removes directory, removes PATH entry, and optionally cleans data.

---

## 3. WiX Windows MSI Installer Lifecycle

### WiX Specification Highlights
- **UpgradeCode**: `{9D7E2B45-7E3A-4F8A-B31F-79A5E2E39401}` (Stable across 2.x releases for automatic major upgrade support).
- **ProductCode**: Dynamic `Product Id="*"` generated per build.
- **Install Scope**: `perMachine` to `%ProgramFiles%\Apeireth\`.
- **Start Menu Shortcuts**:
  - Shortcut located at `%ProgramData%\Microsoft\Windows\Start Menu\Programs\Apeireth\Apeireth OS CLI.lnk`.
  - Application Icon mapped via `ARPPRODUCTICON` from `packaging/msi/icon.ico`.
  - Dedicated Uninstall shortcut.
- **Environment Configuration**:
  - `APEIRETH_HOME` set to `[INSTALLDIR]data`.
  - `APEIRETH_CONFIG` set to `[INSTALLDIR]config\config.toml`.
  - `PATH` appended with `[INSTALLDIR]bin`.
- **Windows Service**:
  - Registered as `ApeirethOS` (`LocalSystem`, auto-start, arguments `serve`).
  - Automatically started on install, stopped on uninstall.
- **Clean Uninstall**:
  - Complete removal of `BIN`, `CONFIG`, `SHARE`, `LOG`, `DATA`, `INSTALLDIR`, registry keys, and Start Menu folder.

### MSI Commands

#### 1. Interactive & Silent Installation
```powershell
# Interactive UI
msiexec /i apeireth-2.0.0-rc.1-windows-x86_64.msi

# Silent Installation
msiexec /i apeireth-2.0.0-rc.1-windows-x86_64.msi /qn /norestart

# With Logging
msiexec /i apeireth-2.0.0-rc.1-windows-x86_64.msi /qn /l*v install.log
```

#### 2. Major Upgrade
When installing a subsequent version (e.g. `2.0.1` or `2.1.0`):
```powershell
msiexec /i apeireth-2.0.1-windows-x86_64.msi /qn
```
WiX `<MajorUpgrade>` automatically removes previous versions while preserving configuration and data.

#### 3. Repair / Maintenance
```powershell
# Re-installs missing or corrupted files/shortcuts
msiexec /f apeireth-2.0.0-rc.1-windows-x86_64.msi
```

#### 4. Clean Uninstallation
```powershell
# Via MSI file
msiexec /x apeireth-2.0.0-rc.1-windows-x86_64.msi /qn

# Or via automation script
.\packaging\msi\uninstall-msi.ps1
```

---

## 4. Scoop Package Manager Lifecycle

### Manifest Integration
- Manifest located at `packaging/scoop/apeireth.json`.
- Manages binaries: `apeireth.exe` and `apeireth-serve`.
- Persists user data in `data/`.

### Commands
```powershell
# Add bucket
scoop bucket add apeireth https://github.com/apeireth/scoop-bucket

# Install
scoop install apeireth

# Upgrade
scoop update apeireth

# Uninstall
scoop uninstall apeireth
```

---

## 5. Tauri Desktop Companion Lifecycle

### Architecture & Config
- Configured in `frontend/companion-desktop/src-tauri/tauri.conf.json`.
- MSI Bundle:
  - UpgradeCode: `B4A6C589-3543-4C4C-8D19-2D8C6FEA3F74`.
  - Languages: `zh-CN`, `en-US`.
- NSIS Bundle:
  - Mode: `perMachine`.
  - Multi-language support.

### Lifecycle Commands
```powershell
# Stage all assets and frontend
.\packaging\desktop\stage-desktop.ps1

# Build Desktop MSI & NSIS
.\packaging\desktop\build-desktop.ps1
```

---

## 6. Zero Hardcoded Path Guarantee

All packaging scripts, WiX templates, and configurations strictly adhere to:
- No dev-machine drive letters (e.g. `D:\...` or `H:\...`).
- Relative script references via `$PSScriptRoot\..\..`.
- Standard Windows environment variables (`%ProgramFiles%`, `%USERPROFILE%`, `%LOCALAPPDATA%`, `[INSTALLDIR]`).
- Verification enforced via `packaging/test-packaging.ps1`.
