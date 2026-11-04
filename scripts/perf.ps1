# 桌面性能预算测量脚本。
#
# 用法：pwsh scripts/perf.ps1 [-Runs 20]
#
# 指标与阈值对应宪法第 V 条的桌面预算（1.18.0），并与
# specs/001-min-action-core/plan.md 的复杂度追踪表一致：
#   启动到可交互、空闲 CPU、常驻内存。
# 测量方式遵循宪法第 V 条：同一台机器、同一组输入、重复 20 次取 p95，不用平均值代替。

param(
    [int]$Runs = 20,
    [string]$Binary = "target\release\ngy-min-action.exe"
)

$ErrorActionPreference = "Stop"

function Get-P95 {
    param([double[]]$Samples)
    if ($Samples.Count -eq 0) { return 0 }
    $sorted = $Samples | Sort-Object
    $index = [Math]::Ceiling($sorted.Count * 0.95) - 1
    return $sorted[[Math]::Max($index, 0)]
}

function Measure-Startup {
    param([int]$Count)
    $samples = @()
    for ($i = 0; $i -lt $Count; $i++) {
        $stopwatch = [Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $Binary -PassThru
        # 主窗口句柄出现即视为「可交互」，避免把渲染首帧算成可用。
        while ($process.MainWindowHandle -eq 0 -and -not $process.HasExited) {
            Start-Sleep -Milliseconds 10
            $process.Refresh()
        }
        $stopwatch.Stop()
        $samples += $stopwatch.Elapsed.TotalMilliseconds
        Stop-Process -Id $process.Id -Force
        Start-Sleep -Milliseconds 300
    }
    return $samples
}

function Measure-IdleCpu {
    param([int]$Seconds)
    $process = Start-Process -FilePath $Binary -PassThru
    Start-Sleep -Seconds 10
    $process.Refresh()
    $startCpu = $process.TotalProcessorTime.TotalSeconds
    $stopwatch = [Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $Seconds
    $process.Refresh()
    $stopwatch.Stop()
    $elapsed = [Math]::Max($stopwatch.Elapsed.TotalSeconds, 1)
    $cpuSeconds = $process.TotalProcessorTime.TotalSeconds - $startCpu
    $cores = [Environment]::ProcessorCount
    Stop-Process -Id $process.Id -Force
    return ($cpuSeconds / $elapsed / $cores) * 100
}

function Measure-ResidentMemory {
    param([int]$Count)
    $samples = @()
    $process = Start-Process -FilePath $Binary -PassThru
    Start-Sleep -Seconds 10
    for ($i = 0; $i -lt $Count; $i++) {
        $process.Refresh()
        $samples += $process.WorkingSet64 / 1MB
        Start-Sleep -Seconds 3
    }
    Stop-Process -Id $process.Id -Force
    return $samples
}

if (-not (Test-Path $Binary)) {
    Write-Error "找不到 $Binary；先执行 cargo build --release"
}

Write-Host "启动到可交互（毫秒）"
$startup = Measure-Startup -Count $Runs
"  p95 = {0:N0} ms（预算 < 1000）" -f (Get-P95 -Samples $startup)

Write-Host "空闲 CPU（无操作 60 秒平均，按全部核心归一）"
$idle = Measure-IdleCpu -Seconds 60
"  {0:N2}%（预算 < 1）" -f $idle

Write-Host "常驻内存（MB，静置 10 秒后每 3 秒采样）"
$memory = Measure-ResidentMemory -Count $Runs
"  p95 = {0:N1} MB（预算 < 250）" -f (Get-P95 -Samples $memory)

Write-Host ""
Write-Host "交互响应与后台驻留内存尚未纳入门禁，原因见宪法第 V 条；"
Write-Host "跨日弹窗延迟由 tests/notify_test.rs 的 Clock 注入覆盖，人工抽验代替计时。"
