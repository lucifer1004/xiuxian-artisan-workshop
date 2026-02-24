#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[test]
fn test_symbol_index_search_performance() {
    const SYMBOL_COUNT: usize = 5000;

    let mut index = SymbolIndex::new();

    // Add many symbols to the index
    for i in 0..SYMBOL_COUNT {
        index.add_symbols(
            &format!("crate_{}", i % 10),
            &[ExternalSymbol {
                name: format!("SymbolName{}", i),
                kind: if i % 5 == 0 {
                    SymbolKind::Struct
                } else if i % 5 == 1 {
                    SymbolKind::Function
                } else if i % 5 == 2 {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Trait
                },
                file: PathBuf::from(format!("file_{}.rs", i % 100)),
                line: i,
                crate_name: format!("crate_{}", i % 10),
            }],
        );
    }

    // Benchmark search
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let results = index.search("SymbolName", 50);
        assert!(!results.is_empty());
    }
    let elapsed = start.elapsed();

    // Should complete 100 searches quickly
    let max_duration = std::time::Duration::from_millis(500);
    assert!(
        elapsed < max_duration,
        "Symbol search took {:.2}ms for {} symbols, expected < 500ms",
        elapsed.as_secs_f64() * 1000.0,
        SYMBOL_COUNT
    );

    println!(
        "Symbol index search: {} symbols, 100 searches = {:.2}ms",
        SYMBOL_COUNT,
        elapsed.as_secs_f64() * 1000.0
    );
}
