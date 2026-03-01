#!/usr/bin/env pwsh
<#
.SYNOPSIS
GitHub Secrets 和本地环境变量管理工具

.DESCRIPTION
用于验证、检查、测试和轮换 DEEPSEEK_API_KEY 的交互式工具

.PARAMETER Action
执行的操作: verify, check, test, rotate (默认: verify)

.EXAMPLE
.\secret-management.ps1 -Action verify
.\secret-management.ps1 -Action check
.\secret-management.ps1 -Action test
.\secret-management.ps1 -Action rotate

.NOTES
必须使用 PowerShell Core 7+ 或 PowerShell 5.1+
#>

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet('verify', 'check', 'test', 'rotate')]
    [string]$Action = 'verify'
)

$ErrorActionPreference = 'Continue'

# ============================================
# 颜色定义
# ============================================
$Colors = @{
    'Green'   = "`e[32m"
    'Red'     = "`e[31m"
    'Yellow'  = "`e[33m"
    'Blue'    = "`e[34m"
    'Cyan'    = "`e[36m"
    'Reset'   = "`e[0m"
}

function Write-Status {
    param([string]$Message, [string]$Status)
    
    $StatusColor = switch ($Status) {
        'success' { $Colors['Green'] }
        'error'   { $Colors['Red'] }
        'warning' { $Colors['Yellow'] }
        'info'    { $Colors['Cyan'] }
        default   { $Colors['Reset'] }
    }
    
    $symbols = @{
        'success' = '✅'
        'error'   = '❌'
        'warning' = '⚠️'
        'info'    = 'ℹ️'
    }
    
    Write-Host "$StatusColor$($symbols[$Status]) $Message$($Colors['Reset'])"
}

# ============================================
# Action 1: verify - GitHub Secrets 设置验证
# ============================================
function Invoke-Verify {
    Write-Host "`n$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])🔐 GitHub Secrets 配置指南$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])`n"
    
    $repos = @(
        'ai-lib-rust',
        'ai-lib-python',
        'ai-lib-ts'
    )
    
    Write-Host "为以下仓库配置 DEEPSEEK_API_KEY:`n"
    
    foreach ($repo in $repos) {
        Write-Status "仓库: $repo" 'info'
    }
    
    Write-Host "`n$($Colors['Yellow'])步骤 1: 访问 GitHub Settings$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "1. 打开 https://github.com/yourname/$repo"
    Write-Host "2. 点击 Settings 标签"
    Write-Host "3. 左侧菜单: Secrets and variables → Actions"
    Write-Host ""
    
    Write-Host "$($Colors['Yellow'])步骤 2: 创建 Secret$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "1. 点击 'New repository secret'"
    Write-Host "2. 名称: DEEPSEEK_API_KEY"
    Write-Host "3. 值: sk-xxxxxxxxxxxxxxxxxxxx (你的实际 API 密钥)"
    Write-Host "4. 点击 'Add secret'"
    Write-Host ""
    
    Write-Host "$($Colors['Yellow'])步骤 3: 验证配置$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "✓ DEEPSEEK_API_KEY 出现在 Secret 列表"
    Write-Host "✓ 值显示为 • (隐藏)"
    Write-Host "✓ 可以编辑或删除"
    Write-Host ""
    
    Write-Host "$($Colors['Cyan'])📋 配置清单$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "□ ai-lib-rust:   DEEPSEEK_API_KEY 配置"
    Write-Host "□ ai-lib-python: DEEPSEEK_API_KEY 配置"
    Write-Host "□ ai-lib-ts:     DEEPSEEK_API_KEY 配置"
    Write-Host "□ 所有 3 个仓库都配置完成"
    Write-Host ""
    
    Write-Status "验证步骤完成" 'success'
    Write-Host "下一步: 运行 'check' 验证本地环境`n"
}

# ============================================
# Action 2: check - 本地环境检查
# ============================================
function Invoke-Check {
    Write-Host "`n$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])🔍 本地环境变量检查$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])`n"
    
    # 检查 .env 文件
    $envFile = '.env'
    Write-Host "1️⃣  检查 $envFile 文件"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if (Test-Path $envFile) {
        Write-Status ".env 文件存在" 'success'
        
        # 读取文件内容
        $envContent = Get-Content $envFile | Select-String 'DEEPSEEK_API_KEY'
        if ($envContent) {
            Write-Status "DEEPSEEK_API_KEY 已配置于 .env" 'success'
            $keyExists = $true
        } else {
            Write-Status "DEEPSEEK_API_KEY 未在 .env 中找到" 'warning'
        }
    } else {
        Write-Status ".env 文件不存在" 'warning'
        Write-Host "  创建方式:"
        Write-Host "  echo 'DEEPSEEK_API_KEY=sk-xxxxxx' > .env"
        Write-Host ""
    }
    
    # 检查系统环境变量
    Write-Host "2️⃣  检查系统环境变量"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if ($env:DEEPSEEK_API_KEY) {
        Write-Status "系统环境变量已设置" 'success'
        $masked = $env:DEEPSEEK_API_KEY.Substring(0, 5) + "..." + $env:DEEPSEEK_API_KEY.Substring(-5)
        Write-Host "  值: $masked (已屏蔽)`n"
    } else {
        Write-Status "系统环境变量未设置" 'warning'
        Write-Host "  设置方式 (临时):"
        Write-Host "  `$env:DEEPSEEK_API_KEY = 'sk-xxxxxx'`n"
    }
    
    # 检查 .gitignore
    Write-Host "3️⃣  检查 .gitignore 安全性"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if (Test-Path '.gitignore') {
        $gitignoreContent = Get-Content '.gitignore'
        if ($gitignoreContent -match '\.env') {
            Write-Status ".env 在 .gitignore 中 (安全)" 'success'
        } else {
            Write-Status ".env 未在 .gitignore 中 (不安全!)" 'error'
            Write-Host "  添加到 .gitignore:"
            Write-Host "  echo '.env' >> .gitignore`n"
        }
    } else {
        Write-Status ".gitignore 不存在" 'warning'
    }
    
    # 检查 Git 历史
    Write-Host "4️⃣  检查 Git 历史中的泄露"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    $leakedKeys = git log --all -S 'sk-' --source -- '*.md' '*.txt' '*.env' 2>/dev/null | wc -l
    
    if ($leakedKeys -eq 0 -or $null -eq $leakedKeys) {
        Write-Status "Git 历史中未发现泄露的密钥" 'success'
    } else {
        Write-Status "检测到可能的密钥泄露!" 'error'
        Write-Host "  运行: git log --all -S 'sk-' --source`n"
    }
    
    Write-Host "$($Colors['Cyan'])📋 检查汇总$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    if ($env:DEEPSEEK_API_KEY -and (Test-Path '.env')) {
        Write-Status "总体状态: 就绪" 'success'
    } else {
        Write-Status "总体状态: 需要配置" 'warning'
    }
    Write-Host ""
}

# ============================================
# Action 3: test - API 连接测试
# ============================================
function Invoke-Test {
    Write-Host "`n$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])🧪 Deepseek API 连接测试$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])`n"
    
    # 首先加载 .env，如果存在
    if (Test-Path '.env') {
        Write-Host "📂 加载 .env 文件..."
        Get-Content '.env' | ForEach-Object {
            if ($_ -match '^\s*([^=]+)=(.*)$') {
                $key = $matches[1]
                $value = $matches[2]
                Set-Item -Path env: -Name $key -Value $value
            }
        }
        Write-Status ".env 已加载" 'success'
    }
    
    # 检查 API Key 是否存在
    if (-not $env:DEEPSEEK_API_KEY) {
        Write-Status "错误: DEEPSEEK_API_KEY 未设置" 'error'
        Write-Host "  请先运行: `$env:DEEPSEEK_API_KEY = 'sk-xxxxx'`n"
        return
    }
    
    Write-Host "1️⃣  测试 API 连接"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "端点: https://api.deepseek.com/v1/chat/completions"
    Write-Host "方法: POST (模型列表请求)`n"
    
    try {
        $headers = @{
            'Authorization' = "Bearer $($env:DEEPSEEK_API_KEY)"
            'Content-Type'  = 'application/json'
        }
        
        $payload = @{
            'model'    = 'deepseek-chat'
            'messages' = @(@{
                'role'    = 'user'
                'content' = 'test'
            })
            'max_tokens' = 10
        } | ConvertTo-Json
        
        Write-Host "📤 发送请求..."
        $response = Invoke-WebRequest `
            -Uri 'https://api.deepseek.com/v1/chat/completions' `
            -Method 'POST' `
            -Headers $headers `
            -Body $payload `
            -TimeoutSec 10 `
            -ErrorAction Stop
        
        Write-Status "API 连接成功!" 'success'
        Write-Host "  状态码: $($response.StatusCode)"
        Write-Host "  响应大小: $($response.Content.Length) 字节`n"
        
    } catch [System.Net.Http.HttpRequestException] {
        Write-Status "HTTP 错误: $($_.Exception.Message)" 'error'
        Write-Host "  可能原因:"
        Write-Host "  • API 密钥无效"
        Write-Host "  • 网络不可达"
        Write-Host "  • API 端点变更`n"
        
    } catch [System.TimeoutException] {
        Write-Status "超时: 请求花费超过 10 秒" 'error'
        Write-Host "  可能原因:"
        Write-Host "  • 网络延迟"
        Write-Host "  • 服务器响应缓慢`n"
        
    } catch {
        Write-Status "错误: $($_.Exception.Message)" 'error'
    }
    
    Write-Host "2️⃣  密钥有效性检查"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    $keyLength = $env:DEEPSEEK_API_KEY.Length
    $keyFormat = $env:DEEPSEEK_API_KEY.Substring(0, 3)
    
    if ($keyFormat -eq 'sk-') {
        Write-Status "密钥格式正确 (sk-...)" 'success'
    } else {
        Write-Status "密钥格式错误，应以 'sk-' 开头" 'error'
    }
    
    Write-Host "  长度: $keyLength 字符"
    Write-Host "  格式: $keyFormat*** (已屏蔽) ***`n"
    
    Write-Host "3️⃣  网络路由检查"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    try {
        $ping = Test-NetConnection -ComputerName 'api.deepseek.com' -Port 443 -WarningAction SilentlyContinue
        if ($ping.TcpTestSucceeded) {
            Write-Status "可以连接到 api.deepseek.com:443" 'success'
        } else {
            Write-Status "无法连接到 api.deepseek.com:443" 'error'
            Write-Host "  可能需要代理或防火墙配置`n"
        }
    } catch {
        Write-Host "  (网络测试跳过)`n"
    }
    
    Write-Host "$($Colors['Cyan'])测试完成$($Colors['Reset'])`n"
}

# ============================================
# Action 4: rotate - 密钥轮换
# ============================================
function Invoke-Rotate {
    Write-Host "`n$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])🔄 密钥轮换步骤$($Colors['Reset'])"
    Write-Host "$($Colors['Blue'])═══════════════════════════════════════$($Colors['Reset'])`n"
    
    Write-Host "根据策略，应按月轮换 API 密钥以维护安全性。`n"
    
    Write-Host "$($Colors['Yellow'])步骤 1: 在 Deepseek 控制面板生成新密钥$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "1. 登录 https://platform.deepseek.com"
    Write-Host "2. 进入 API Keys 部分"
    Write-Host "3. 点击 'Create new key'"
    Write-Host "4. 复制新的 sk-xxxxxx 密钥"
    Write-Host "5. 妥善保存旧密钥（用于回滚）`n"
    
    Write-Host "$($Colors['Yellow'])步骤 2: 更新 GitHub Secrets$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "对于每个仓库 (ai-lib-rust, ai-lib-python, ai-lib-ts):"
    Write-Host ""
    Write-Host "1. GitHub.com 仓库主页"
    Write-Host "2. Settings → Secrets and variables → Actions"
    Write-Host "3. 找到 DEEPSEEK_API_KEY"
    Write-Host "4. 点击编辑 (Edit)"
    Write-Host "5. 用新密钥替换旧密钥"
    Write-Host "6. 点击 Update secret`n"
    
    Write-Host "$($Colors['Yellow'])步骤 3: 更新本地 .env 文件$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    $newKey = Read-Host "输入新的 API 密钥 (或按 Enter 跳过)"
    
    if ($newKey) {
        if (Test-Path '.env') {
            $content = Get-Content '.env'
            $updated = $content -replace '^DEEPSEEK_API_KEY=.*', "DEEPSEEK_API_KEY=$newKey"
            Set-Content '.env' $updated
            Write-Status ".env 已更新" 'success'
        } else {
            Add-Content '.env' "DEEPSEEK_API_KEY=$newKey"
            Write-Status ".env 已创建" 'success'
        }
        
        $env:DEEPSEEK_API_KEY = $newKey
        Write-Host "  新密钥已加载到环境`n"
    } else {
        Write-Host "  (跳过本地更新)`n"
    }
    
    Write-Host "$($Colors['Yellow'])步骤 4: 验证新密钥$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "运行测试来验证新密钥是否有效:"
    Write-Host ""
    Write-Host "  .\secret-management.ps1 -Action test`n"
    
    Write-Host "$($Colors['Yellow'])步骤 5: 撤销旧密钥$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "等待所有系统使用新密钥运行成功后:"
    Write-Host "1. 登录 https://platform.deepseek.com"
    Write-Host "2. 找到旧的 API Key"
    Write-Host "3. 点击删除或禁用"
    Write-Host "4. 确认撤销`n"
    
    Write-Host "$($Colors['Cyan'])📋 轮换清单$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "□ 生成新 API 密钥 (Deepseek 平台)"
    Write-Host "□ 更新 GitHub Secrets (3 个仓库)"
    Write-Host "□ 更新本地 .env 文件"
    Write-Host "□ 验证新密钥 (运行 test)"
    Write-Host "□ 撤销旧密钥 (Deepseek 平台)"
    Write-Host "□ 更新密钥轮换日期"
    Write-Host ""
    
    Write-Host "$($Colors['Cyan'])🗓️  轮换计划$($Colors['Reset'])"
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "• 计划轮换: 每月一次"
    Write-Host "• 紧急轮换: 立即 (如果泄露)"
    Write-Host "• 最后轮换: $(Get-Date -Format 'yyyy-MM-dd')"
    Write-Host "• 下一轮换: $(Get-Date -Date (Get-Date).AddMonths(1) -Format 'yyyy-MM-dd')`n"
}

# ============================================
# 主程序
# ============================================

Write-Host "$($Colors['Cyan'])GitHub Secrets & 密钥管理工具$($Colors['Reset'])`n"
Write-Host "操作: $Action`n"

switch ($Action) {
    'verify' { Invoke-Verify }
    'check'  { Invoke-Check }
    'test'   { Invoke-Test }
    'rotate' { Invoke-Rotate }
    default  { Write-Status "未知操作: $Action" 'error' }
}

Write-Host "$($Colors['Green'])✅ 完成$($Colors['Reset'])`n"
