//! AI-Protocol connectivity report
//!
//! For each provider that has a local API key (env var `{PROVIDER_ID}_API_KEY`),
//! runs a basic_usage-style chat request and records success or error.
//! Outputs a report and error analysis with links to provider API references.
//!
//! Usage:
//!   AI_PROTOCOL_DIR=/path/to/ai-protocol cargo run --example connectivity_report
//! Or with keys set:
//!   DEEPSEEK_API_KEY="..." GROQ_API_KEY="..." cargo run --example connectivity_report

use ai_lib_rust::{AiClient, Message};
use std::env;
use std::time::Instant;

/// One (provider_id, model_id) to test. model_id format: "provider/model-name"
/// (model-name is the key in ai-protocol v1/models/{provider}.yaml or cross-provider key).
/// Aligned with ai-protocol v1/providers and v1/models.
const PROVIDER_MODELS: &[(&str, &str)] = &[
    ("deepseek", "deepseek/deepseek-chat"),
    ("groq", "groq/llama-3.1-8b-instant"),
    ("nvidia", "nvidia/meta/llama3-70b"),
    ("minimax", "minimax/abab6.5s-chat"),
    ("zhipu", "zhipu/glm-4-plus"),
    ("openai", "openai/gpt-4o-mini"),
    ("anthropic", "anthropic/claude-3-5-sonnet"),
    (
        "together",
        "together/together/meta-llama/Llama-3-70b-chat-hf",
    ),
    ("qwen", "qwen/qwen-turbo"),
    ("moonshot", "moonshot/moonshot-v1-8k"),
    ("mistral", "mistral/mistral-small-latest"),
    ("cohere", "cohere/command-r-plus-08-2024"),
    (
        "fireworks",
        "fireworks/accounts/fireworks/models/llama-v3p1-70b-instruct",
    ),
    (
        "deepinfra",
        "deepinfra/meta-llama/Meta-Llama-3.1-70B-Instruct",
    ),
    ("lepton", "lepton/meta-llama/Meta-Llama-3.1-70B-Instruct"),
    ("sensenova", "sensenova/sensenova-v1"),
    ("spark", "spark/general"),
    ("yi", "yi/yi-large"),
    ("baichuan", "baichuan/Baichuan2-Turbo"),
    ("doubao", "doubao/doubao-1-5-pro-32k"),
    ("hunyuan", "hunyuan/hunyuan-lite"),
    ("tiangong", "tiangong/sky-pro"),
    ("siliconflow", "siliconflow/SF-Llama3.1-8B-Instruct"),
    ("perplexity", "perplexity/llama-3.1-sonar-small-128k-online"),
    ("replicate", "replicate/meta/meta-llama-3-70b-instruct"),
    ("cerebras", "cerebras/llama3.1-8b"),
    ("ai21", "ai21/j3-mini"),
    ("baidu", "baidu/ernie-bot-4"),
];

fn env_key_for_provider(provider_id: &str) -> String {
    format!("{}_API_KEY", provider_id.to_uppercase())
}

fn has_api_key(provider_id: &str) -> bool {
    env::var(env_key_for_provider(provider_id)).is_ok()
}

#[derive(Debug)]
struct Row {
    provider_id: String,
    model_id: String,
    status: String,
    duration_ms: u64,
    error_message: Option<String>,
}

/// Provider API reference URLs for error analysis.
fn api_reference_url(provider_id: &str) -> &'static str {
    match provider_id {
        "deepseek" => "https://platform.deepseek.com/docs",
        "groq" => "https://console.groq.com/docs",
        "nvidia" => "https://docs.api.nvidia.com/nim/reference/llm-apis",
        "minimax" => "https://platform.minimaxi.com/document/ChatCompletion%20v2",
        "zhipu" => "https://open.bigmodel.cn/dev/api",
        "openai" => "https://platform.openai.com/docs/api-reference",
        "anthropic" => "https://docs.anthropic.com/en/api",
        "openrouter" => "https://openrouter.ai/docs",
        "together" => "https://docs.together.ai/reference/chat-completions",
        "qwen" => "https://help.aliyun.com/zh/model-studio/developer-reference/api-details",
        "moonshot" => "https://platform.moonshot.cn/docs",
        "mistral" => "https://docs.mistral.ai/api/",
        "cohere" => "https://docs.cohere.com/reference/chat",
        "fireworks" => "https://docs.fireworks.ai/api-reference",
        "deepinfra" => "https://deepinfra.com/docs",
        "lepton" => "https://www.lepton.ai/docs",
        "sensenova" => "https://platform.sensenova.cn/doc",
        "spark" => "https://www.xfyun.cn/doc/spark/Web.html",
        "yi" => "https://platform.lingyiwanwu.com/docs",
        "baichuan" => "https://platform.baichuan-ai.com/docs",
        "doubao" => "https://www.volcengine.com/docs/82379",
        "hunyuan" => "https://cloud.tencent.com/document/product/1729",
        "tiangong" => "https://help.aliyun.com/zh/model-studio/developer-reference/tongyi-qianwen",
        "siliconflow" => "https://docs.siliconflow.cn",
        "perplexity" => "https://docs.perplexity.ai/api-docs",
        "replicate" => "https://replicate.com/docs/reference/http",
        "cerebras" => "https://docs.cerebras.ai/inference-api",
        "ai21" => "https://docs.ai21.com/reference/complete",
        "baidu" => "https://cloud.baidu.com/doc/WENXINWORKSHOP/index.html",
        _ => "https://github.com/hiddenpath/ai-protocol",
    }
}

fn classify_error(err: &str) -> (&'static str, &'static str) {
    let e = err.to_lowercase();
    // Check 429 / quota before 401 so "429 + insufficient_quota" is not misclassified as auth
    if e.contains("429")
        || e.contains("rate_limited")
        || e.contains("insufficient_quota")
        || e.contains("quota") && (e.contains("exceeded") || e.contains("billing"))
    {
        (
            "429 / 限流或配额",
            "请求过于频繁或配额用尽；可重试或检查账单/升级配额。",
        )
    } else if e.contains("401")
        || e.contains("authentication")
        || e.contains("invalid api key")
        || e.contains("authorized_error")
    {
        ("401 / 认证失败", "检查 API Key 是否正确、是否过期；确认环境变量名与 manifest 中 token_env 一致（如 DEEPSEEK_API_KEY）。")
    } else if e.contains("404") || e.contains("not found") || e.contains("not_found") {
        (
            "404 / 资源不存在",
            "确认 manifest 中 endpoint path 与厂商文档一致；模型 ID 是否在厂商当前可用列表中。",
        )
    } else if e.contains("500")
        || e.contains("503")
        || e.contains("server_error")
        || e.contains("overloaded")
    {
        ("5xx / 服务端错误", "厂商服务暂时不可用；稍后重试。")
    } else if e.contains("timeout") || e.contains("timed out") {
        ("超时", "增大 AI_HTTP_TIMEOUT_SECS 或检查网络。")
    } else if e.contains("protocol") || (e.contains("not found") && e.contains("provider")) {
        (
            "协议/Manifest 未找到",
            "设置 AI_PROTOCOL_DIR 指向 ai-protocol 目录；或确认该 provider 在 ai-protocol 中存在。",
        )
    } else {
        ("其他", "对照厂商 API 文档核对请求格式与参数。")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?),
        )
        .with_target(false)
        .try_init();

    // Auto-detect ai-protocol in common relative paths (cross-platform)
    if env::var("AI_PROTOCOL_DIR").is_err() && env::var("AI_PROTOCOL_PATH").is_err() {
        let candidates = ["ai-protocol", "../ai-protocol", "../../ai-protocol"];
        for p in &candidates {
            if std::path::Path::new(p).join("v1/providers").exists() {
                env::set_var("AI_PROTOCOL_DIR", *p);
                println!("Using AI_PROTOCOL_DIR={}\n", p);
                break;
            }
        }
    }

    let to_test: Vec<(&str, &str)> = PROVIDER_MODELS
        .iter()
        .copied()
        .filter(|(pid, _)| has_api_key(pid))
        .collect();

    if to_test.is_empty() {
        println!("No API keys found. Set env vars such as:");
        println!("  DEEPSEEK_API_KEY, GROQ_API_KEY, NVIDIA_API_KEY, MINIMAX_API_KEY, ZHIPU_API_KEY, OPENAI_API_KEY, ...");
        println!("(Convention: {{PROVIDER_ID}}_API_KEY, e.g. deepseek -> DEEPSEEK_API_KEY)\n");
        println!("Skipped providers (no key):");
        for (pid, mid) in PROVIDER_MODELS {
            println!("  {} ({})", pid, mid);
        }
        return Ok(());
    }

    println!("=== AI-Protocol connectivity test ===\n");
    println!("Testing {} provider(s) with API key set.\n", to_test.len());

    let mut rows: Vec<Row> = Vec::new();
    for (provider_id, model_id) in &to_test {
        let start = Instant::now();
        let (status, error_message) = match run_one(model_id).await {
            Ok(()) => ("OK".to_string(), None),
            Err(e) => {
                let msg = e.to_string();
                ("FAIL".to_string(), Some(msg))
            }
        };
        let duration_ms = start.elapsed().as_millis() as u64;
        rows.push(Row {
            provider_id: (*provider_id).to_string(),
            model_id: (*model_id).to_string(),
            status,
            duration_ms,
            error_message,
        });
    }

    // Print table
    println!("| Provider   | Model ID (abbr)           | Status | Duration(ms) |");
    println!("|------------|----------------------------|--------|--------------|");
    for r in &rows {
        let mid_short = if r.model_id.len() > 26 {
            format!("{}...", &r.model_id[..23])
        } else {
            r.model_id.clone()
        };
        let err_short = r
            .error_message
            .as_ref()
            .map(|s| {
                if s.len() > 60 {
                    format!("{}...", &s[..57])
                } else {
                    s.clone()
                }
            })
            .unwrap_or_default();
        println!(
            "| {:<10} | {:<26} | {:<6} | {:<12} |",
            r.provider_id, mid_short, r.status, r.duration_ms
        );
        if !err_short.is_empty() {
            println!("|            | {} |", err_short);
        }
    }

    // Error analysis
    let failures: Vec<_> = rows.iter().filter(|r| r.status == "FAIL").collect();
    if !failures.is_empty() {
        println!("\n=== Error analysis (fact-check against provider API reference) ===\n");
        for r in &failures {
            let err = r.error_message.as_deref().unwrap_or("");
            let (kind, suggestion) = classify_error(err);
            let url = api_reference_url(&r.provider_id);
            println!("[{}] {} — {}", r.provider_id, kind, suggestion);
            println!("  API reference: {}", url);
            println!("  Raw error: {}\n", err);
        }
    }

    let ok_count = rows.iter().filter(|r| r.status == "OK").count();
    println!(
        "=== Summary: {} OK, {} FAIL ===\n",
        ok_count,
        failures.len()
    );

    if let Ok(path) = env::var("CONNECTIVITY_REPORT_OUT") {
        let mut out = String::new();
        out.push_str("=== AI-Protocol connectivity report ===\n\n");
        for r in &rows {
            out.push_str(&format!(
                "{} | {} | {} | {} ms\n",
                r.provider_id, r.model_id, r.status, r.duration_ms
            ));
            if let Some(e) = &r.error_message {
                out.push_str(&format!("  Error: {}\n", e));
            }
        }
        out.push_str(&format!(
            "\nSummary: {} OK, {} FAIL\n",
            ok_count,
            failures.len()
        ));
        std::fs::write(&path, out).ok();
        println!("Report written to {}", path);
    }
    Ok(())
}

async fn run_one(model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = AiClient::new(model_id).await?;
    let messages = vec![
        Message::system("You are a helpful assistant. Reply in one short sentence."),
        Message::user("Say hello."),
    ];
    let _ = client
        .chat()
        .messages(messages)
        .temperature(0.1)
        .max_tokens(100)
        .execute()
        .await?;
    Ok(())
}
