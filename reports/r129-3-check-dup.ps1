Set-Location "Apeireth-rust/reports/"
$files = Get-ChildItem -Name "agent-r129-3-*" | Where-Object { $_.EndsWith(".md") }
foreach ($f in $files) {
  $item = Get-Item $f
  Write-Output "File: [$f]"
  Write-Output "  Created: $($item.CreationTime)"
  Write-Output "  Modified: $($item.LastWriteTime)"
  Write-Output "  Size: $($item.Length) bytes"
}
