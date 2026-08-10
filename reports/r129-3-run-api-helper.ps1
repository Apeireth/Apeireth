$env:APEIRETH_API_KEY = "r129-3-verify-test-key-not-real"
Set-Location "Apeireth-rust/"
cargo run --bin apeireth-api --offline -- --help 2>&1 | Tee-Object -FilePath "reports/agent-r129-3-cargo-run-api-env-2026-08-11.log" | Select-Object -Last 15
"---EXIT $LASTEXITCODE---"
