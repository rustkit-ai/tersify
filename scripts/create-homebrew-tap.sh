#!/usr/bin/env bash
# Creates the rustkit-ai/homebrew-tap repo locally and pushes the tersify formula.
# Run this once. Requires: git, curl, and a GitHub personal access token with 'repo' scope.
#
# Usage:
#   GITHUB_TOKEN=ghp_xxx bash scripts/create-homebrew-tap.sh
#
set -euo pipefail

OWNER="rustkit-ai"
TAP_REPO="homebrew-tap"
FORMULA_NAME="tersify"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
  echo "Error: set GITHUB_TOKEN=<your personal access token>"
  exit 1
fi

echo "Creating GitHub repo ${OWNER}/${TAP_REPO}..."

# Create repo via GitHub API (skip if already exists)
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github.v3+json" \
  https://api.github.com/orgs/${OWNER}/repos \
  -d "{\"name\":\"${TAP_REPO}\",\"description\":\"Homebrew tap for rustkit-ai tools\",\"public\":true}")

if [[ "$HTTP_STATUS" == "422" ]]; then
  echo "Repo already exists — continuing."
elif [[ "$HTTP_STATUS" != "201" ]]; then
  echo "GitHub API returned $HTTP_STATUS — check your token and org name."
  exit 1
fi

# Clone or init the tap repo locally
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

git clone "https://${GITHUB_TOKEN}@github.com/${OWNER}/${TAP_REPO}.git" "$TMPDIR" 2>/dev/null \
  || (cd "$TMPDIR" && git init && git remote add origin "https://${GITHUB_TOKEN}@github.com/${OWNER}/${TAP_REPO}.git")

mkdir -p "$TMPDIR/Formula"

# Compute SHA256 of the source tarball for this version
URL="https://github.com/${OWNER}/${FORMULA_NAME}/archive/refs/tags/v${VERSION}.tar.gz"
echo "Computing SHA256 for v${VERSION}..."
SHA256=$(curl -fsSL "$URL" | shasum -a 256 | cut -d' ' -f1)

cat > "$TMPDIR/Formula/${FORMULA_NAME}.rb" <<FORMULA
class $(echo "${FORMULA_NAME}" | sed 's/./\u&/') < Formula
  desc "Universal LLM context compressor — pipe anything, get token-optimized output"
  homepage "https://github.com/${OWNER}/${FORMULA_NAME}"
  url "${URL}"
  sha256 "${SHA256}"
  license "MIT"
  head "https://github.com/${OWNER}/${FORMULA_NAME}.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  def post_install
    system "\#{bin}/${FORMULA_NAME}", "install", "--all"
  rescue StandardError
    nil
  end

  test do
    assert_match version.to_s, shell_output("\#{bin}/${FORMULA_NAME} --version")
    (testpath/"test.rs").write("// comment\nfn main() {}\n")
    output = shell_output("\#{bin}/${FORMULA_NAME} \#{testpath}/test.rs")
    refute_match "// comment", output
  end
end
FORMULA

cd "$TMPDIR"
git add Formula/
git config user.email "ci@rustkit-ai" 2>/dev/null || true
git config user.name "rustkit-ai" 2>/dev/null || true
git diff --cached --quiet || git commit -m "${FORMULA_NAME} ${VERSION}"
git push -u origin HEAD:main

echo ""
echo "✓ Homebrew tap ready!"
echo ""
echo "  brew tap ${OWNER}/tap"
echo "  brew install ${FORMULA_NAME}"
echo ""
echo "  Next releases will auto-update via the GitHub Actions 'homebrew' job."
echo "  Add HOMEBREW_TAP_ENABLED=true to your repo variables and"
echo "  HOMEBREW_TAP_TOKEN (PAT with repo scope) to your repo secrets."
