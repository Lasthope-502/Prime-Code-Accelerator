use regex::Regex;
use std::fs;
use anyhow::Result;
use once_cell::sync::Lazy;

pub struct NodePattern {
    pub name: &'static str,
    pub description: &'static str,
    pub rust_fn: &'static str,
    pub category: &'static str,
    pub regex: &'static str,
}

static PATTERNS: Lazy<Vec<(NodePattern, Regex)>> = Lazy::new(|| {
    let defs = vec![
        NodePattern {
            name: "sum_of_squares_loop",
            description: "'total += i*i' style accumulation loop detected",
            rust_fn: "sumOfSquares",
            category: "numeric_loop",
            regex: r"for\s*\([^)]*\)\s*\{[^}]*?(\w+)\s*\+=\s*\w+\s*\*\s*\w+",
        },
        NodePattern {
            name: "range_sum_loop",
            description: "simple accumulation loop (total += i) detected",
            rust_fn: "fastRangeSum",
            category: "numeric_loop",
            regex: r"for\s*\([^)]*\)\s*\{[^}]*?(\w+)\s*\+=\s*\w+\s*;",
        },
        NodePattern {
            name: "nested_loop_matrix",
            description: "Nested for-loops detected — O(n^2)+ candidate",
            rust_fn: "matrixMultiply",
            category: "nested_loop",
            regex: r"for\s*\([^)]*\)\s*\{[^{}]*for\s*\([^)]*\)\s*\{",
        },
        NodePattern {
            name: "string_concat_loop",
            description: "String += concatenation in loop (slow in hot path)",
            rust_fn: "fastStringJoin",
            category: "string_ops",
            regex: r#"for\s*\([^)]*\)\s*\{[^}]*?\w+\s*\+=\s*(`|'|"|\w+\.toString)"#,
        },
        NodePattern {
            name: "array_push_loop",
            description: "array.push() inside loop detected",
            rust_fn: "fastCollect",
            category: "collection_ops",
            regex: r"for\s*\([^)]*\)\s*\{[^}]*?\.push\(",
        },
        NodePattern {
            name: "object_counting_loop",
            description: "Object property counting pattern (frequency map) detected",
            rust_fn: "fastWordCount",
            category: "collection_ops",
            regex: r"\w+\[\w+\]\s*=\s*\(\w+\[\w+\]\s*\|\|\s*0\)\s*\+\s*1",
        },
    ];

    defs.into_iter()
        .filter_map(|d| Regex::new(d.regex).ok().map(|re| (d, re)))
        .collect()
});

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub name: String,
    pub description: String,
    pub rust_fn: String,
    pub category: String,
}

/// Converts a V8 Inspector "file://" URL into a usable native filesystem path.
/// Handles Windows drive-letter paths (file:///F:/...) and percent-encoded
/// characters (e.g. %20 for spaces), both of which broke plain string matching.
fn file_url_to_path(url: &str) -> String {
    let mut s = url.to_string();

    if let Some(stripped) = s.strip_prefix("file://") {
        s = stripped.to_string();
    }

    // Windows paths end up as "/F:/dir/file.js" after stripping file:// —
    // strip the extra leading slash before the drive letter.
    let bytes = s.as_bytes();
    if bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':' {
        s = s[1..].to_string();
    }

    percent_decode(&s)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(hex);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub fn detect_patterns(filename: &str, line: u64) -> Result<Vec<PatternMatch>> {
    let clean_path = file_url_to_path(filename);
    let content = match fs::read_to_string(&clean_path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = (line.saturating_sub(5)) as usize;
    let end = ((line + 20) as usize).min(lines.len());
    if start >= end {
        return Ok(vec![]);
    }
    let context = lines[start..end].join("\n");

    let mut matches = vec![];
    for (def, re) in PATTERNS.iter() {
        if re.is_match(&context) {
            matches.push(PatternMatch {
                name: def.name.to_string(),
                description: def.description.to_string(),
                rust_fn: def.rust_fn.to_string(),
                category: def.category.to_string(),
            });
        }
    }

    matches.sort_by_key(|m| match m.category.as_str() {
        "nested_loop" => 0,
        "numeric_loop" => 1,
        "string_ops" => 2,
        _ => 3,
    });

    Ok(matches)
}