# 09:30 强制收尾 — 停所有 agent + 写最终报告
# 主人 8/10 02:55 派, 09:30 收尾, 09:30-10:00 总结

# 5 个 agent task id
$tasks = @(
  @{ name='A'; id='bg_04619ef7-c97e-4a26-914f-5e9ee91c6fcf' },
  @{ name='B'; id='bg_19cc8b54-e806-43d7-9cb4-5303b1769762' },
  @{ name='C'; id='bg_cfb86c96-3a6d-4561-8547-60063a3d57ac' },
  @{ name='D-1'; id='bg_4e91577a-cc5b-433e-a2ce-951d476d5ae6' },
  @{ name='D-2'; id='bg_67724c09-a28f-4e9a-8155-ae643a305ead' }
)

foreach ($t in $tasks) {
  $r = mavis task query --task_id $t.id 2>&1
  if ($r -match 'running') {
    Write-Host "Stopping $($t.name) ($($t.id))..."
    mavis task stop --task_id $t.id 2>&1
  } elseif ($r -match 'succeeded') {
    Write-Host "$($t.name) already succeeded"
  } else {
    Write-Host "$($t.name) state: $r"
  }
}

# 跑最后一次 baseline
powershell -NoProfile -ExecutionPolicy Bypass -File '.openclaw\workspace\promethean\Apeireth-rust\scripts\verify-baseline.ps1'
