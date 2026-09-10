; Apeireth Companion — NSIS installer hooks (Tauri `installerHooks`).
;
; Why this file exists: the Tauri-generated uninstaller only checks the MAIN
; binary (companion-desktop.exe) for a running process. The bundled canonical
; sidecar apeireth.exe can also be running:
;   - spawned by the app itself (backend_supervisor), or
;   - started standalone by a user (`apeireth gateway serve` — the installed
;     CLI exposes it, and INSTALL.md documents it).
; Uninstalling with a live sidecar fails to delete the locked apeireth.exe,
; yet still removes the registry entry and exits 0: a silent half-uninstall.
; Verified live by the packaged install E2E on 2026-09-08 (uninstall exit 0,
; install dir + apeireth.exe left behind, uninstall registry entry gone).
;
; Fix: mirror the generated main-binary guard (same kill semantics, same
; silent-mode behavior: /S kills without prompting; if the kill fails the
; uninstaller aborts non-zero instead of leaving a half-removed install).
;
; Order matters: kill the APP first — its supervisor can respawn the sidecar —
; then kill the sidecar. The generated script runs its own main-binary check
; right after this hook; the duplicate main check here is a deliberate no-op.
;
; Update mode (/UPDATE): the generated script kills the main binary
; unconditionally in that flow too, so the sidecar must be handled identically
; or updates would never replace the locked apeireth.exe.
!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  !insertmacro CheckIfAppIsRunning "apeireth.exe" "${PRODUCTNAME} (apeireth sidecar)"
!macroend
