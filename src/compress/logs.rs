use std::collections::HashMap;

/// Deduplicate log lines by normalising variable parts:
/// timestamps, hex IDs, UUIDs, and large numbers become placeholders.
/// Identical normalised lines are collapsed with an occurrence count.
pub fn compress(input: &str) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    // Use a Vec to preserve original line order
    let mut order: Vec<String> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let key = normalise(line);
        let entry = counts.entry(key.clone()).or_insert(0);
        if *entry == 0 {
            order.push(key.clone());
        }
        *entry += 1;
    }

    order
        .into_iter()
        .map(|key| {
            let n = counts[&key];
            if n > 1 { format!("[×{n}] {key}") } else { key }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalise(line: &str) -> String {
    use regex_lite::Regex;

    // ISO 8601 timestamps
    let ts =
        Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?").unwrap();
    let s = ts.replace_all(line, "<ts>");

    // UUIDs
    let uuid = Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let s = uuid.replace_all(&s, "<uuid>");

    // Hex strings (≥8 chars)
    let hex = Regex::new(r"\b[0-9a-fA-F]{8,}\b").unwrap();
    let s = hex.replace_all(&s, "<hex>");

    // Large integers (≥4 digits)
    let num = Regex::new(r"\b\d{4,}\b").unwrap();
    num.replace_all(&s, "<n>").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_repeated_lines() {
        let logs = "2024-01-01T10:00:00Z INFO ping\n\
                    2024-01-01T10:00:01Z INFO ping\n\
                    2024-01-01T10:00:02Z INFO ping\n\
                    2024-01-01T10:00:03Z ERROR fail\n";
        let out = compress(logs);
        assert!(out.contains("[×3]"));
        assert!(out.contains("INFO ping"));
        assert!(out.contains("ERROR fail"));
        assert!(!out.contains("[×1]"));
    }

    #[test]
    fn unique_lines_have_no_count() {
        let logs = "INFO a\nINFO b\nINFO c\n";
        let out = compress(logs);
        assert!(!out.contains("[×"));
    }
}
