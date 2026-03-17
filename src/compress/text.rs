use std::collections::HashSet;

/// Remove duplicate sentences and trim excessive blank lines from plain text.
pub fn compress(input: &str) -> String {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<&str> = Vec::new();
    let mut blank_run = 0usize;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run == 1 {
                out.push("");
            }
            continue;
        }
        blank_run = 0;
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(line);
        }
    }

    out.join("\n")
}
