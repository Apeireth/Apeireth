$env:APEIRETH_LLM_BACKEND = "scripted"
Set-Location "Apeireth-rust"
cargo run --bin apeireth-api -- --help 2>&1 | Out-File -FilePath "reports\agent-r154-3-cargo-run-api-help-2026-08-11.log" -Encoding UTF8
Write-Output "exit=$LASTEXITCODE"
