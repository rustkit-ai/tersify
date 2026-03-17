//! Shared helpers used across compress submodules.

/// Count `{` and `}` in `line`, ignoring braces inside string literals and `//` comments.
pub(super) fn brace_counts(line: &str) -> (i32, i32) {
    let chars: Vec<char> = line.chars().collect();
    let mut opens = 0i32;
    let mut closes = 0i32;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            break; // line comment — stop
        }
        if c == '"' || c == '\'' || c == '`' {
            let q = c;
            i += 1;
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        match c {
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
        i += 1;
    }
    (opens, closes)
}

/// Return the leading whitespace slice of `line`.
pub(super) fn leading_ws(line: &str) -> &str {
    let trimmed_len = line.trim_start().len();
    &line[..line.len() - trimmed_len]
}

/// Collapse runs of >1 blank line to exactly one, strip leading/trailing blank lines.
pub(super) fn collapse_blank_lines(input: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut blanks = 0usize;
    for line in input.lines() {
        if line.trim().is_empty() {
            blanks += 1;
            if blanks == 1 {
                out.push("");
            }
        } else {
            blanks = 0;
            out.push(line);
        }
    }
    while out.first().is_some_and(|l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}
