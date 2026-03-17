/// Compress a unified git diff for LLM consumption:
/// - Keep file headers (diff --git, ---, +++)
/// - Keep changed lines (+ and -)
/// - Drop context lines (lines starting with a space)
/// - Drop hunk position headers (@@ ... @@) — the LLM doesn't need line numbers
pub fn compress(input: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut current_file: Option<&str> = None;

    for line in input.lines() {
        if line.starts_with("diff --git") {
            current_file = Some(line);
            out.push(line);
        } else if line.starts_with("--- ") || line.starts_with("+++ ") {
            out.push(line);
        } else if line.starts_with("@@") {
            // Replace with a compact separator — drop position numbers
            out.push("---");
        } else if line.starts_with('+') || line.starts_with('-') {
            let _ = current_file; // file context already emitted
            out.push(line);
        }
        // Context lines (leading space) are intentionally dropped
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_context_lines() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 use std::io;
-fn old() {}
+fn new() {}
 fn other() {}
";
        let out = compress(diff);
        assert!(out.contains("+fn new()"));
        assert!(out.contains("-fn old()"));
        assert!(!out.contains(" fn other")); // context line removed
        assert!(!out.contains("@@ -1,5")); // hunk header removed
        assert!(out.contains("---")); // separator kept
    }

    #[test]
    fn preserves_file_headers() {
        let diff = "diff --git a/foo.rs b/foo.rs\n--- a/foo.rs\n+++ b/foo.rs\n";
        let out = compress(diff);
        assert!(out.contains("diff --git"));
        assert!(out.contains("--- a/foo.rs"));
        assert!(out.contains("+++ b/foo.rs"));
    }
}
