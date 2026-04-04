#!/usr/bin/env pwsh
# Tavily Tool Calling Example - Quick Start Guide
# 快速开始指南

Write-Host "🚀 AI-Lib-Rust Tool Calling Example Setup" -ForegroundColor Green
Write-Host ""

# 检查API密钥
$has_deepseek = Test-Path env:DEEPSEEK_API_KEY
$has_openai = Test-Path env:OPENAI_API_KEY
$has_anthropic = Test-Path env:ANTHROPIC_API_KEY
$has_groq = Test-Path env:GROQ_API_KEY

if (-not ($has_deepseek -or $has_openai -or $has_anthropic -or $has_groq)) {
    Write-Host "❌ No API keys found! Please set one of:" -ForegroundColor Red
    Write-Host ""
    Write-Host "  使用 DeepSeek (推荐):" -ForegroundColor Yellow
    Write-Host '  $env:DEEPSEEK_API_KEY="your_api_key"'
    Write-Host ""
    Write-Host "  使用 OpenAI:" -ForegroundColor Yellow
    Write-Host '  $env:OPENAI_API_KEY="your_api_key"'
    Write-Host ""
    Write-Host "  使用 Anthropic:" -ForegroundColor Yellow
    Write-Host '  $env:ANTHROPIC_API_KEY="your_api_key"'
    Write-Host ""
    Write-Host "  使用 Groq:" -ForegroundColor Yellow
    Write-Host '  $env:GROQ_API_KEY="your_api_key"'
    Write-Host ""
    exit 1
}

Write-Host "✅ API Key Setup:" -ForegroundColor Green
if ($has_deepseek) { Write-Host "  ✓ DEEPSEEK_API_KEY" -ForegroundColor Green }
if ($has_openai) { Write-Host "  ✓ OPENAI_API_KEY" -ForegroundColor Green }
if ($has_anthropic) { Write-Host "  ✓ ANTHROPIC_API_KEY" -ForegroundColor Green }
if ($has_groq) { Write-Host "  ✓ GROQ_API_KEY" -ForegroundColor Green }
Write-Host ""

Write-Host "📝 Usage Examples:" -ForegroundColor Cyan
Write-Host ""

Write-Host "1. 使用自动检测 (Auto-detect):" -ForegroundColor Yellow
Write-Host "   cargo run --example tavily_tool_calling" -ForegroundColor Gray
Write-Host ""

Write-Host "2. 使用特定提供商 (Specify provider):" -ForegroundColor Yellow
Write-Host "   cargo run --example tavily_tool_calling -- --provider openai" -ForegroundColor Gray
Write-Host ""

Write-Host "3. 启用调试日志 (Enable debug logging):" -ForegroundColor Yellow
Write-Host '   $env:RUST_LOG="ai_lib_rust::pipeline=debug"' -ForegroundColor Gray
Write-Host "   cargo run --example tavily_tool_calling" -ForegroundColor Gray
Write-Host ""

Write-Host "🎯 Running Now..." -ForegroundColor Cyan
Write-Host ""

cargo run --example tavily_tool_calling
