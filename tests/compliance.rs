//! AI-Protocol compliance test runner.
//!
//! Discovers and executes declarative YAML test cases from the ai-protocol
//! tests/compliance directory. Supports error_classification tests and can be
//! extended for other test types.

use ai_lib_rust::client::classify_error_from_response;
use serde::Deserialize;
use serde_yaml::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct TestCase {
    suite: String,
    name: String,
    id: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    setup: Option<TestSetup>,
    input: TestInput,
    expected: TestExpected,
}

#[derive(Debug, Deserialize)]
struct TestSetup {
    provider: Option<String>,
    manifest_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestInput {
    #[serde(rename = "type")]
    test_type: String,
    #[serde(default)]
    http_status: Option<u16>,
    #[serde(default)]
    response_body: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct TestExpected {
    error_code: String,
    error_name: String,
    retryable: bool,
    fallbackable: bool,
}

impl Default for TestExpected {
    fn default() -> Self {
        Self {
            error_code: String::new(),
            error_name: String::new(),
            retryable: false,
            fallbackable: false,
        }
    }
}

fn compliance_dir() -> PathBuf {
    if let Ok(dir) = env::var("COMPLIANCE_DIR") {
        return PathBuf::from(dir);
    }

    // Try common sibling layouts:
    // 1. ../ai-protocol/tests/compliance  (same parent dir)
    // 2. ../../ai-protocol/tests/compliance  (ai-protocol at grandparent level)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../ai-protocol/tests/compliance"),
        manifest_dir.join("../../ai-protocol/tests/compliance"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    // Fallback
    manifest_dir
        .parent()
        .unwrap()
        .join("ai-protocol/tests/compliance")
}

fn discover_yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(discover_yaml_files(&path));
            } else if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn parse_test_cases(content: &str) -> Vec<TestCase> {
    // Normalize line endings to LF (handle Windows CRLF)
    let content = content.replace("\r\n", "\n");
    let mut cases = Vec::new();
    // Use serde_yaml's multi-document support via Deserializer
    for document in serde_yaml::Deserializer::from_str(&content) {
        match TestCase::deserialize(document) {
            Ok(tc) => cases.push(tc),
            Err(e) => {
                // Not all documents are test cases (e.g., comments-only blocks);
                // log a debug warning and continue.
                eprintln!("  [WARN] Skipped non-test-case document: {}", e);
            }
        }
    }
    cases
}

fn run_error_classification(tc: &TestCase) -> Result<(), Vec<String>> {
    let http_status = tc.input.http_status.expect("error_classification requires http_status");
    let response_body = tc.input.response_body.as_ref();
    let actual = classify_error_from_response(http_status, response_body);

    let mut failures = Vec::new();

    if actual.code() != tc.expected.error_code {
        failures.push(format!(
            "error_code: expected {}, got {}",
            tc.expected.error_code,
            actual.code()
        ));
    }
    if actual.name() != tc.expected.error_name {
        failures.push(format!(
            "error_name: expected {}, got {}",
            tc.expected.error_name,
            actual.name()
        ));
    }
    if actual.retryable() != tc.expected.retryable {
        failures.push(format!(
            "retryable: expected {}, got {}",
            tc.expected.retryable,
            actual.retryable()
        ));
    }
    if actual.fallbackable() != tc.expected.fallbackable {
        failures.push(format!(
            "fallbackable: expected {}, got {}",
            tc.expected.fallbackable,
            actual.fallbackable()
        ));
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

#[test]
fn compliance_error_classification() {
    let compliance_dir = compliance_dir();
    if !compliance_dir.exists() {
        eprintln!(
            "[SKIP] Compliance directory does not exist: {}",
            compliance_dir.display()
        );
        eprintln!("       Set COMPLIANCE_DIR to override, or run from workspace with ai-protocol.");
        return;
    }

    let error_class_dir = compliance_dir.join("cases/02-error-classification");
    if !error_class_dir.exists() {
        eprintln!(
            "[SKIP] Error classification cases dir does not exist: {}",
            error_class_dir.display()
        );
        return;
    }

    let yaml_files = discover_yaml_files(&error_class_dir);
    let mut passed = 0u32;
    let mut failed = 0u32;

    for file in yaml_files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  [WARN] Could not read {}: {}", file.display(), e);
                continue;
            }
        };

        let cases = parse_test_cases(&content);
        for tc in cases {
            if tc.input.test_type != "error_classification" {
                continue;
            }

            match run_error_classification(&tc) {
                Ok(()) => {
                    println!("  [PASS] {} ({}) - {}", tc.id, tc.name, tc.expected.error_code);
                    passed += 1;
                }
                Err(failures) => {
                    println!("  [FAIL] {} ({})", tc.id, tc.name);
                    for f in &failures {
                        println!("         {}", f);
                    }
                    failed += 1;
                }
            }
        }
    }

    println!("\n--- Compliance summary ---");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    assert_eq!(
        failed, 0,
        "{} error_classification compliance test(s) failed",
        failed
    );
}
