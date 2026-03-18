class Tersify < Formula
  desc "Universal LLM context compressor — pipe anything, get token-optimized output"
  homepage "https://github.com/rustkit-ai/tersify"
  url "https://github.com/rustkit-ai/tersify/archive/refs/tags/v0.3.3.tar.gz"
  sha256 "0fc51141572dd7439284cd5a6089922b510eedb04596b7bc63ef7d4281a478f4"
  license "MIT"
  head "https://github.com/rustkit-ai/tersify.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  def post_install
    system "#{bin}/tersify", "install", "--all"
  rescue StandardError
    nil # don't fail if no editors detected
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tersify --version")
    # Compression smoke test
    (testpath/"test.rs").write("// comment\nfn main() {\n    println!(\"hello\");\n}\n")
    output = shell_output("#{bin}/tersify #{testpath}/test.rs")
    assert_match "fn main()", output
    refute_match "// comment", output
  end
end
