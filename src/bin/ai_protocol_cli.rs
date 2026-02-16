//! AI-Protocol CLI — 协议清单验证、兼容性检查、能力查询的命令行工具
//!
//! Usage:
//!   ai-protocol-cli validate [--dir <path>]       Validate all provider manifests
//!   ai-protocol-cli info <provider>                Show provider capabilities
//!   ai-protocol-cli check-compat <manifest>        Check runtime compatibility
//!   ai-protocol-cli list                           List all available providers

use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "validate" => cmd_validate(&args[2..]),
        "info" => cmd_info(&args[2..]),
        "list" => cmd_list(&args[2..]),
        "check-compat" => cmd_check_compat(&args[2..]),
        "version" | "--version" | "-V" => cmd_version(),
        "help" | "--help" | "-h" => print_usage(),
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!(
        r#"ai-protocol-cli — AI-Protocol 命令行工具

USAGE:
    ai-protocol-cli <COMMAND> [OPTIONS]

COMMANDS:
    validate [--dir <path>]     Validate provider manifests (V1 + V2)
    info <provider>             Show provider capabilities and configuration
    list [--dir <path>]         List all available provider manifests
    check-compat <manifest>     Check runtime feature compatibility
    version                     Show version information
    help                        Show this help message

ENVIRONMENT:
    AI_PROTOCOL_DIR             Protocol repository root path"#
    );
}

fn cmd_version() {
    println!(
        "ai-protocol-cli {} (ai-lib-rust {})",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
    );
}

fn resolve_protocol_dir(args: &[String]) -> PathBuf {
    // Check --dir flag
    for (i, arg) in args.iter().enumerate() {
        if arg == "--dir" {
            if let Some(path) = args.get(i + 1) {
                return PathBuf::from(path);
            }
        }
    }
    // Check environment variable
    if let Ok(dir) = std::env::var("AI_PROTOCOL_DIR") {
        return PathBuf::from(dir);
    }
    // Try common relative paths
    for candidate in &["../ai-protocol", "../../ai-protocol", "ai-protocol"] {
        let p = PathBuf::from(candidate);
        if p.join("v2").join("providers").exists() || p.join("v1").join("providers").exists() {
            return p;
        }
    }
    eprintln!("Error: Cannot find protocol directory. Set AI_PROTOCOL_DIR or use --dir.");
    std::process::exit(1);
}

fn cmd_validate(args: &[String]) {
    let dir = resolve_protocol_dir(args);
    println!("Protocol directory: {}", dir.display());
    println!();

    let mut total = 0u32;
    let mut passed = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Validate V2 providers
    let v2_dir = dir.join("v2").join("providers");
    if v2_dir.exists() {
        println!("=== V2 Provider Manifests ===");
        validate_yaml_dir(&v2_dir, &mut total, &mut passed, &mut errors, "V2");
    }

    // Validate V1 providers
    let v1_dir = dir.join("v1").join("providers");
    if v1_dir.exists() {
        println!("\n=== V1 Provider Manifests ===");
        validate_yaml_dir(&v1_dir, &mut total, &mut passed, &mut errors, "V1");
    }

    // Validate schemas
    let schemas_dir = dir.join("schemas").join("v2");
    if schemas_dir.exists() {
        println!("\n=== V2 Schemas ===");
        validate_json_dir(&schemas_dir, &mut total, &mut passed, &mut errors);
    }

    // Summary
    println!("\n=== Summary ===");
    println!("{passed}/{total} files valid");
    if errors.is_empty() {
        println!("All files pass validation.");
    } else {
        println!("{} error(s):", errors.len());
        for err in &errors {
            println!("  {err}");
        }
        std::process::exit(1);
    }
}

fn validate_yaml_dir(
    dir: &PathBuf,
    total: &mut u32,
    passed: &mut u32,
    errors: &mut Vec<String>,
    version_label: &str,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", dir.display());
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        *total += 1;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        print!("  [{version_label}] {name}... ");

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_yaml::from_str::<serde_json::Value>(&content) {
                Ok(val) => {
                    // V2: check required fields
                    if version_label == "V2" {
                        if let Some(obj) = val.as_object() {
                            if !obj.contains_key("id") || !obj.contains_key("protocol_version") {
                                println!("WARN (missing id/protocol_version)");
                                errors.push(format!("{name}: missing required V2 fields"));
                                return;
                            }
                        }
                    }
                    println!("OK");
                    *passed += 1;
                }
                Err(e) => {
                    println!("FAIL");
                    errors.push(format!("{name}: YAML parse error: {e}"));
                }
            },
            Err(e) => {
                println!("FAIL");
                errors.push(format!("{name}: read error: {e}"));
            }
        }
    }
}

fn validate_json_dir(dir: &PathBuf, total: &mut u32, passed: &mut u32, errors: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", dir.display());
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        *total += 1;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        print!("  {name}... ");

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(_) => {
                    println!("OK");
                    *passed += 1;
                }
                Err(e) => {
                    println!("FAIL");
                    errors.push(format!("{name}: JSON parse error: {e}"));
                }
            },
            Err(e) => {
                println!("FAIL");
                errors.push(format!("{name}: read error: {e}"));
            }
        }
    }
}

fn cmd_info(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: ai-protocol-cli info <provider>");
        std::process::exit(1);
    }
    let provider = &args[0];
    let dir = resolve_protocol_dir(&args[1..]);

    // Try V2 first, then V1
    let v2_path = dir.join("v2").join("providers").join(format!("{provider}.yaml"));
    let v1_path = dir.join("v1").join("providers").join(format!("{provider}.yaml"));

    let (path, version) = if v2_path.exists() {
        (v2_path, "V2")
    } else if v1_path.exists() {
        (v1_path, "V1")
    } else {
        eprintln!("Provider '{provider}' not found in V1 or V2 directories.");
        std::process::exit(1);
    };

    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {e}", path.display());
        std::process::exit(1);
    });

    let val: serde_json::Value = serde_yaml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Invalid YAML: {e}");
        std::process::exit(1);
    });

    println!("Provider: {provider} [{version}]");
    println!("File: {}", path.display());
    println!();

    if let Some(obj) = val.as_object() {
        if let Some(id) = obj.get("id") {
            println!("  ID: {}", id.as_str().unwrap_or("?"));
        }
        if let Some(name) = obj.get("name") {
            println!("  Name: {}", name.as_str().unwrap_or("?"));
        }
        if let Some(pv) = obj.get("protocol_version") {
            println!("  Protocol Version: {}", pv.as_str().unwrap_or("?"));
        }
        if let Some(status) = obj.get("status") {
            println!("  Status: {}", status.as_str().unwrap_or("?"));
        }

        // Capabilities
        if let Some(caps) = obj.get("capabilities") {
            println!("\n  Capabilities:");
            if let Some(req) = caps.get("required").and_then(|v| v.as_array()) {
                let names: Vec<&str> = req.iter().filter_map(|v| v.as_str()).collect();
                println!("    Required: {}", names.join(", "));
            }
            if let Some(opt) = caps.get("optional").and_then(|v| v.as_array()) {
                let names: Vec<&str> = opt.iter().filter_map(|v| v.as_str()).collect();
                println!("    Optional: {}", names.join(", "));
            }
        }

        // MCP
        if let Some(mcp) = obj.get("mcp") {
            println!("\n  MCP:");
            if let Some(client) = mcp.get("client") {
                let supported = client.get("supported").and_then(|v| v.as_bool()).unwrap_or(false);
                println!("    Client supported: {supported}");
                if supported {
                    if let Some(transports) = client.get("transports").and_then(|v| v.as_array()) {
                        let t: Vec<&str> = transports.iter().filter_map(|v| v.as_str()).collect();
                        println!("    Transports: {}", t.join(", "));
                    }
                }
            }
        }

        // Computer Use
        if let Some(cu) = obj.get("computer_use") {
            println!("\n  Computer Use:");
            let supported = cu.get("supported").and_then(|v| v.as_bool()).unwrap_or(false);
            println!("    Supported: {supported}");
            if supported {
                if let Some(impl_style) = cu.get("implementation").and_then(|v| v.as_str()) {
                    println!("    Implementation: {impl_style}");
                }
                if let Some(status) = cu.get("status").and_then(|v| v.as_str()) {
                    println!("    Status: {status}");
                }
            }
        }

        // Multimodal
        if let Some(mm) = obj.get("multimodal") {
            println!("\n  Multimodal:");
            if let Some(input) = mm.get("input") {
                let mut modalities = Vec::new();
                for (name, cfg) in input.as_object().into_iter().flatten() {
                    if cfg.get("supported").and_then(|v| v.as_bool()).unwrap_or(false) {
                        modalities.push(name.as_str());
                    }
                }
                if !modalities.is_empty() {
                    println!("    Input: {}", modalities.join(", "));
                }
            }
            if let Some(output) = mm.get("output") {
                let mut modalities = Vec::new();
                for (name, cfg) in output.as_object().into_iter().flatten() {
                    if name == "text" {
                        continue;
                    }
                    if cfg.get("supported").and_then(|v| v.as_bool()).unwrap_or(false) {
                        modalities.push(name.as_str());
                    }
                }
                if !modalities.is_empty() {
                    println!("    Output: text, {}", modalities.join(", "));
                }
            }
        }
    }
}

fn cmd_list(args: &[String]) {
    let dir = resolve_protocol_dir(args);
    println!("Protocol directory: {}", dir.display());
    println!();

    let mut providers: HashMap<String, Vec<String>> = HashMap::new();

    for (version, subdir) in [("V2", "v2"), ("V1", "v1")] {
        let pdir = dir.join(subdir).join("providers");
        if !pdir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&pdir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "yaml" || ext == "yml" {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    providers.entry(name).or_default().push(version.to_string());
                }
            }
        }
    }

    if providers.is_empty() {
        println!("No provider manifests found.");
        return;
    }

    let mut sorted: Vec<_> = providers.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    println!("{:<20} {}", "Provider", "Versions");
    println!("{}", "-".repeat(40));
    for (name, versions) in &sorted {
        println!("{:<20} {}", name, versions.join(", "));
    }
    println!("\nTotal: {} provider(s)", sorted.len());
}

fn cmd_check_compat(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: ai-protocol-cli check-compat <manifest.yaml>");
        std::process::exit(1);
    }
    let path = PathBuf::from(&args[0]);
    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {e}", path.display());
        std::process::exit(1);
    });

    let val: serde_json::Value = serde_yaml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Invalid YAML: {e}");
        std::process::exit(1);
    });

    let obj = val.as_object().unwrap_or_else(|| {
        eprintln!("Manifest must be a YAML object");
        std::process::exit(1);
    });

    let provider = obj.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    println!("Checking compatibility for: {provider}");
    println!();

    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check required fields
    if !obj.contains_key("id") {
        issues.push("Missing required field: id".into());
    }
    if !obj.contains_key("protocol_version") {
        issues.push("Missing required field: protocol_version".into());
    }
    if !obj.contains_key("endpoint") {
        issues.push("Missing required field: endpoint".into());
    }
    if !obj.contains_key("capabilities") {
        issues.push("Missing required field: capabilities".into());
    }

    // Check capabilities for feature requirements
    if let Some(caps) = obj.get("capabilities") {
        let required: Vec<&str> = caps
            .get("required")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let feature_gated = ["mcp_client", "mcp_server", "computer_use", "vision",
                             "audio", "video", "reasoning", "image_generation"];

        println!("  Feature Requirements:");
        for cap in &required {
            if feature_gated.contains(cap) {
                let feature = match *cap {
                    "mcp_client" | "mcp_server" => "mcp",
                    "vision" | "audio" | "video" | "image_generation" => "multimodal",
                    c => c,
                };
                println!("    {cap} -> Rust feature '{feature}', Python extra '{feature}'");
            }
        }
    }

    // Check MCP configuration
    if let Some(mcp) = obj.get("mcp") {
        if let Some(client) = mcp.get("client") {
            if client.get("supported").and_then(|v| v.as_bool()).unwrap_or(false) {
                if client.get("transports").and_then(|v| v.as_array()).map(|a| a.is_empty()).unwrap_or(true) {
                    warnings.push("MCP client enabled but no transports specified".into());
                }
            }
        }
    }

    // Check Computer Use safety
    if let Some(cu) = obj.get("computer_use") {
        if cu.get("supported").and_then(|v| v.as_bool()).unwrap_or(false) {
            if cu.get("safety").is_none() {
                warnings.push("Computer Use enabled but no safety configuration".into());
            }
        }
    }

    // Print results
    println!();
    if !issues.is_empty() {
        println!("  ERRORS ({}):", issues.len());
        for issue in &issues {
            println!("    [ERROR] {issue}");
        }
    }
    if !warnings.is_empty() {
        println!("  WARNINGS ({}):", warnings.len());
        for warn in &warnings {
            println!("    [WARN] {warn}");
        }
    }
    if issues.is_empty() && warnings.is_empty() {
        println!("  All checks passed.");
    }

    if !issues.is_empty() {
        std::process::exit(1);
    }
}
