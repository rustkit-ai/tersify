# This file lives in the main repo for reference.
# The canonical formula is maintained in rustkit-ai/homebrew-tap.
#
# To install:
#   brew tap rustkit-ai/tap
#   brew install tersify
#
# SHA256 values are updated automatically by .github/workflows/release.yml
# on each release.

class Tersify < Formula
  desc "Universal LLM context compressor — pipe anything, get token-optimized output"
  homepage "https://github.com/rustkit-ai/tersify"
  version "PLACEHOLDER_VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/rustkit-ai/tersify/releases/download/v#{version}/tersify-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_DARWIN"
    end

    on_intel do
      url "https://github.com/rustkit-ai/tersify/releases/download/v#{version}/tersify-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_DARWIN"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/rustkit-ai/tersify/releases/download/v#{version}/tersify-aarch64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_LINUX"
    end

    on_intel do
      url "https://github.com/rustkit-ai/tersify/releases/download/v#{version}/tersify-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_X86_64_LINUX"
    end
  end

  def install
    bin.install "tersify"
  end

  def post_install
    system "#{bin}/tersify", "install", "--all"
  rescue StandardError
    nil # don't fail if no editors detected
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tersify --version")
    (testpath/"test.rs").write("// comment\nfn main() {\n    println!(\"hello\");\n}\n")
    output = shell_output("#{bin}/tersify #{testpath}/test.rs")
    assert_match "fn main()", output
    refute_match "// comment", output
  end
end
