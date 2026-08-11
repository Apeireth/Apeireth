$a = @('a','b','c')
$b = @('a','b','d')
$result = Compare-Object $a $b
foreach ($item in $result) {
  Write-Output "SideIndicator: '$($item.SideIndicator)' InputObject: '$($item.InputObject)'"
}
Write-Output "---"
Write-Output "Interpretation: => means in ReferenceObject (first arg), <= means in DifferenceObject (second arg)"
