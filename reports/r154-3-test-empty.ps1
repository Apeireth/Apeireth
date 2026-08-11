function Test-Func {
  $names = @()
  $content = @("hello world", "another line", "no match here")
  foreach ($line in $content) {
    if ($line -match "^\s*pub\s+mod\s+([a-zA-Z_][a-zA-Z0-9_]*)") {
      $names += $Matches[1]
    }
  }
  return ($names | Sort-Object -Unique)
}

$result = Test-Func
Write-Output "result is null: $($null -eq $result)"
Write-Output "result count: $($result.Count)"
$result | ForEach-Object { Write-Output "item: $_" }
