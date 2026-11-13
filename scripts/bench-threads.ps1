<#
.SYNOPSIS
  How fast one --fix-swat run gets on 1 to N threads (M5 in docs/roadmap.md), on Windows.

.DESCRIPTION
  Runs `swatplus-rust run <project> --fix-swat --threads N` for each thread count, a few times
  each, and writes every run's time and a fingerprint of its output tables to a CSV file. The
  fingerprint must be the same for every thread count (the results are the same bit for bit);
  the summary at the end says whether it is.

  Each run writes its tables to a fresh folder under %TEMP%, deleted after it is fingerprinted.
  Close other programs and keep the laptop on mains power while it runs.

.EXAMPLE
  .\bench-threads.ps1 -Exe .\swatplus-rust.exe -Project D:\swat\Batanghari\Scenarios\Default\TxtInOut

.EXAMPLE
  .\bench-threads.ps1 -Exe .\swatplus-rust.exe -Project D:\swat\Pessel\TxtInOut -Threads 1,2,4,8,12 -Repeats 5 -Out pessel.csv
#>
param(
    [Parameter(Mandatory = $true)] [string] $Exe,
    [Parameter(Mandatory = $true)] [string] $Project,
    [int[]] $Threads = @(1, 2, 3, 4, 6, 8, 10, 12),
    [int] $Repeats = 3,
    [string] $Out = "bench-threads.csv"
)

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
$Project = (Resolve-Path $Project).Path

function Get-Fingerprint([string] $dir) {
    # SHA-256 of every table's SHA-256 with its name, in name order.
    $lines = Get-ChildItem -Path $dir -File -Recurse | Sort-Object FullName | ForEach-Object {
        $rel = $_.FullName.Substring($dir.Length).TrimStart('\')
        "$((Get-FileHash -Algorithm SHA256 $_.FullName).Hash) $rel"
    }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))
    $sha = [System.Security.Cryptography.SHA256]::Create()
    return ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace '-', '').Substring(0, 16)
}

$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1)
Write-Host "CPU: $($cpu.Name), $($cpu.NumberOfCores) cores, $($cpu.NumberOfLogicalProcessors) logical"
Write-Host "Program: $Exe"
& $Exe --version
Write-Host "Project: $Project"
Write-Host ""

$rows = @()
# A first run warms the disk cache; its time isn't kept.
$tmp = Join-Path $env:TEMP ("swatbench-" + [guid]::NewGuid())
& $Exe run $Project --out $tmp --fix-swat --threads 1 *> $null
Remove-Item -Recurse -Force $tmp

foreach ($r in 1..$Repeats) {
    # Thread counts in turn within each repeat, so a slow spell spreads over all of them.
    foreach ($t in $Threads) {
        $tmp = Join-Path $env:TEMP ("swatbench-" + [guid]::NewGuid())
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & $Exe run $Project --out $tmp --fix-swat --threads $t *> $null
        $sw.Stop()
        if ($LASTEXITCODE -ne 0) { throw "the run with $t threads failed (exit $LASTEXITCODE)" }
        $fp = Get-Fingerprint $tmp
        Remove-Item -Recurse -Force $tmp
        $s = [math]::Round($sw.Elapsed.TotalSeconds, 3)
        Write-Host ("repeat {0}  threads {1,2}  {2,8:N2} s  {3}" -f $r, $t, $s, $fp)
        $rows += [pscustomobject]@{ threads = $t; repeat = $r; seconds = $s; fingerprint = $fp }
    }
}
$rows | Export-Csv -NoTypeInformation -Path $Out

Write-Host ""
Write-Host "threads  median s  speed-up"
$base = $null
foreach ($t in $Threads) {
    $v = @($rows | Where-Object { $_.threads -eq $t } | ForEach-Object { $_.seconds } | Sort-Object)
    $med = if ($v.Count % 2) { $v[[int][math]::Floor($v.Count / 2)] } else { ($v[$v.Count / 2 - 1] + $v[$v.Count / 2]) / 2 }
    if ($null -eq $base) { $base = $med }
    Write-Host ("{0,7}  {1,8:N2}  {2,8:N2}" -f $t, $med, ($base / $med))
}
$fps = @($rows | ForEach-Object { $_.fingerprint } | Sort-Object -Unique)
if ($fps.Count -eq 1) {
    Write-Host "`nSame tables for every thread count (fingerprint $($fps[0]))."
} else {
    Write-Host "`nTHE TABLES DIFFER between thread counts: $($fps -join ', ')"
}
Write-Host "Written to $Out"
