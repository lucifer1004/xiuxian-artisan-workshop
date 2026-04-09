//! Benchmark tests for tokenization performance.
//!
//! These tests measure the performance of BPE tokenization using tiktoken.
//! The tokenizer uses `OnceLock` caching for optimal performance.

use std::fmt::Write as _;
use std::time::Duration;

fn running_in_ci() -> bool {
    std::env::var_os("CI").is_some()
}

fn ci_adjusted_duration(local: Duration, ci: Duration) -> Duration {
    if running_in_ci() { ci } else { local }
}

fn benchmark_budget(local: Duration, ci: Duration) -> Duration {
    let base = ci_adjusted_duration(local, ci);
    let slack_factor = std::env::var("OMNI_TOKENIZER_BENCH_SLACK_FACTOR")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 1.0)
        .unwrap_or(2.0);
    Duration::from_secs_f64(base.as_secs_f64() * slack_factor)
}

fn warm_up_tokenizer() {
    let _ = xiuxian_tokenizer::count_tokens("warmup");
}

fn generate_test_text(char_count: usize) -> String {
    let words = [
        "hello",
        "world",
        "rust",
        "python",
        "tokenizer",
        "benchmark",
        "performance",
        "optimization",
        "vector",
        "search",
        "database",
        "async",
        "await",
        "parallel",
        "concurrent",
        "memory",
        "cpu",
        "function",
        "class",
        "struct",
        "enum",
        "trait",
        "interface",
    ];

    let mut result = String::with_capacity(char_count);
    let word_count = char_count / 6;

    for i in 0..word_count {
        if i > 0 && i % 10 == 0 {
            result.push('\n');
        } else if i > 0 {
            result.push(' ');
        }
        result.push_str(words[i % words.len()]);
    }

    result
}

fn generate_code_text(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 50);

    for i in 0..line_count {
        let _ = write!(
            content,
            r#"fn function_{i}(arg1: &str, arg2: i32) -> Result<(), Box<dyn std::error::Error>> {{
    let result = process_data(arg1, arg2)?;
    println!("Result: {{}}", result);
    Ok(())
}}

"#
        );
    }

    content
}

fn generate_json_text(entry_count: usize) -> String {
    let mut content = String::with_capacity(entry_count * 100);
    content.push_str("[\n");

    for i in 0..entry_count {
        if i > 0 {
            content.push_str(",\n");
        }
        let _ = write!(
            content,
            r#"  {{
    "id": {i},
    "name": "item_{i}",
    "description": "This is a test item with index {i}",
    "tags": ["test", "benchmark", "performance"],
    "value": {i}.5,
    "active": true
}}"#
        );
    }

    content.push_str("\n]\n");
    content
}

#[test]
fn test_token_counting_performance() {
    const TEXT_SIZE: usize = 10000;
    const ITERATIONS: usize = 20;

    let text = generate_test_text(TEXT_SIZE);
    warm_up_tokenizer();

    let start = std::time::Instant::now();

    for _ in 0..ITERATIONS {
        let count = xiuxian_tokenizer::count_tokens(&text);
        assert!(count > 0);
    }

    let elapsed = start.elapsed();
    let max_duration = benchmark_budget(Duration::from_secs(2), Duration::from_secs(4));
    assert!(
        elapsed < max_duration,
        "Token counting took {:.2}s for {} iterations, expected < {:.2}s",
        elapsed.as_secs_f64(),
        ITERATIONS,
        max_duration.as_secs_f64()
    );
}

#[test]
fn test_large_text_tokenization() {
    const TEXT_SIZE: usize = 100_000;

    let text = generate_test_text(TEXT_SIZE);
    warm_up_tokenizer();

    let start = std::time::Instant::now();
    let count = xiuxian_tokenizer::count_tokens(&text);
    let elapsed = start.elapsed();

    let max_duration = benchmark_budget(Duration::from_millis(800), Duration::from_millis(1500));
    assert!(
        elapsed < max_duration,
        "Large text tokenization took {:.2}ms for {} chars, expected < {:.2}ms",
        elapsed.as_secs_f64() * 1000.0,
        TEXT_SIZE,
        max_duration.as_secs_f64() * 1000.0
    );
    assert!(count > 0);
}

#[test]
fn test_code_tokenization_performance() {
    const LINE_COUNT: usize = 100;

    let code = generate_code_text(LINE_COUNT);
    let start = std::time::Instant::now();

    for _ in 0..10 {
        let count = xiuxian_tokenizer::count_tokens(&code);
        assert!(count > 0);
    }

    let elapsed = start.elapsed();
    let max_duration = benchmark_budget(Duration::from_secs(10), Duration::from_secs(20));
    assert!(
        elapsed < max_duration,
        "Code tokenization took {:.2}s for {} iterations, expected < {:.2}s",
        elapsed.as_secs_f64(),
        10,
        max_duration.as_secs_f64()
    );
}

#[test]
fn test_json_tokenization_performance() {
    const ENTRY_COUNT: usize = 200;

    let json = generate_json_text(ENTRY_COUNT);
    let start = std::time::Instant::now();

    for _ in 0..10 {
        let count = xiuxian_tokenizer::count_tokens(&json);
        assert!(count > 0);
    }

    let elapsed = start.elapsed();
    let max_duration = benchmark_budget(Duration::from_secs(15), Duration::from_secs(30));
    assert!(
        elapsed < max_duration,
        "JSON tokenization took {:.2}s for 10 iterations, expected < {:.2}s",
        elapsed.as_secs_f64(),
        max_duration.as_secs_f64()
    );
}

#[test]
fn test_truncate_performance() {
    const TEXT_SIZE: usize = 5000;
    const MAX_TOKENS: usize = 100;

    let text = generate_test_text(TEXT_SIZE);
    let start = std::time::Instant::now();

    for _ in 0..100 {
        let truncated = xiuxian_tokenizer::truncate(&text, MAX_TOKENS);
        assert!(!truncated.is_empty());
    }

    let elapsed = start.elapsed();
    let max_duration = benchmark_budget(Duration::from_secs(8), Duration::from_secs(12));
    assert!(
        elapsed < max_duration,
        "Truncate took {:.2}s for 100 iterations, expected < {:.2}s",
        elapsed.as_secs_f64(),
        max_duration.as_secs_f64()
    );
}

#[test]
fn test_batch_token_counting() {
    const BATCH_SIZE: usize = 100;
    const TEXT_SIZE: usize = 1000;

    let texts: Vec<String> = (0..BATCH_SIZE)
        .map(|_| generate_test_text(TEXT_SIZE))
        .collect();

    let start = std::time::Instant::now();
    let total_tokens: usize = texts
        .iter()
        .map(|text| xiuxian_tokenizer::count_tokens(text))
        .sum();
    let elapsed = start.elapsed();

    let max_duration = benchmark_budget(Duration::from_secs(4), Duration::from_secs(6));
    assert!(
        elapsed < max_duration,
        "Batch token counting took {:.2}s for {} texts, expected < {:.2}s",
        elapsed.as_secs_f64(),
        BATCH_SIZE,
        max_duration.as_secs_f64()
    );
    assert!(total_tokens > 0);
}

#[test]
fn test_varying_text_sizes() {
    let sizes = [100, 1000, 5000, 10000, 50000];
    warm_up_tokenizer();

    for size in sizes {
        let text = generate_test_text(size);
        let start = std::time::Instant::now();
        let count = xiuxian_tokenizer::count_tokens(&text);
        let elapsed = start.elapsed();
        let max_duration =
            benchmark_budget(Duration::from_millis(800), Duration::from_millis(1300));

        assert!(
            elapsed < max_duration,
            "Tokenization of {} chars took {:.2}ms, expected < {:.2}ms",
            size,
            elapsed.as_secs_f64() * 1000.0,
            max_duration.as_secs_f64() * 1000.0
        );
        assert!(count > 0 || size == 0);
    }
}

#[test]
fn test_token_counting_correctness() {
    assert_eq!(xiuxian_tokenizer::count_tokens("hello world"), 2);
    assert_eq!(xiuxian_tokenizer::count_tokens("hello"), 1);
    assert_eq!(xiuxian_tokenizer::count_tokens(""), 0);

    let code = generate_code_text(10);
    let count = xiuxian_tokenizer::count_tokens(&code);
    assert!(count > 0, "should count some tokens in code");

    let text = generate_test_text(5000);
    let truncated = xiuxian_tokenizer::truncate(&text, 50);
    let truncated_count = xiuxian_tokenizer::count_tokens(&truncated);
    assert!(
        truncated_count <= 50,
        "Truncated text should have <= 50 tokens, got {truncated_count}"
    );
}

#[test]
fn test_token_counter_wrapper() {
    let text = generate_test_text(1000);
    let start = std::time::Instant::now();

    for _ in 0..100 {
        let count = xiuxian_tokenizer::TokenCounter::count_tokens(&text);
        assert!(count > 0);
    }

    let elapsed = start.elapsed();
    let max_duration = benchmark_budget(Duration::from_secs(5), Duration::from_secs(7));
    assert!(
        elapsed < max_duration,
        "TokenCounter wrapper took {:.2}s, expected < {:.2}s",
        elapsed.as_secs_f64(),
        max_duration.as_secs_f64()
    );
}
