# Audit scripts

`orphan-scan.ps1` is the current repository-structure audit. It reads Cargo
metadata, reports pure libraries without normal consumers, and never deletes
code.

`8-promise-audit.sh` is retained as a historical 1.0-release record. Its
LOCKED-crate inventory describes the pre-freeze repository and is not a source
of truth for the current 13-package workspace.
