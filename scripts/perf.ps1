# 桌面性能预算测量脚本（草案，收尾见 issue #1）。
#
# 用法：pwsh scripts/perf.ps1 [-Runs 20]
#
# 输出 p95 指标，与 specs/001-min-action-core/plan.md 的复杂度追踪表对应：
#   启动到可交互、跨日弹窗延迟、交互响应、空闲 CPU、常驻内存、后台驻留内存。
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

function Measure-ResidentMemory {
    param([int]$Count)
    $samples = @()
    $process = Start-Process -FilePath $Binary -PassThru
    Start-Sleep -Seconds 5
    for ($i = 0; $i -lt $Count; $i++) {
        $process.Refresh()
        $samples += $process.WorkingSet64 / 1MB
        Start-Sleep -Milliseconds 500
    }
    Stop-Process -Id $process.Id -Force
    return $samples
}

if (-not (Test-Path $Binary)) {
    Write-Error "找不到 $Binary；先执行 cargo build --release"
}

Write-Host "启动到可交互（毫秒）"
$startup = Measure-Startup -Count $Runs
"  p95 = {0:N0} ms（预算 < 2000）" -f (Get-P95 -Samples $startup)

Write-Host "常驻内存（MB）"
$memory = Measure-ResidentMemory -Count $Runs
"  p95 = {0:N1} MB（预算 < 150）" -f (Get-P95 -Samples $memory)

Write-Host ""
Write-Host "跨日弹窗延迟与交互响应需在程序运行中改变系统日期或用自动化点击测量，"
Write-Host "把结果与上表一并填入 issue #1，作为修订宪法第 V 条的依据。"
