class TreesitterMcp < Formula
  desc "AST-first MCP server for token-efficient code analysis"
  homepage "https://github.com/Christoph/treesitter-mcp"
  url "https://github.com/Christoph/treesitter-mcp.git",
      tag: "v0.6.0",
      revision: "1b3dedc5f46c2d281434b04a1327ad276dec0d60"
  license "MIT"
  head "https://github.com/Christoph/treesitter-mcp.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_predicate bin/"treesitter-mcp", :exist?
    assert_predicate bin/"treesitter-mcp", :executable?
    output = shell_output("strings #{bin}/treesitter-mcp | grep -m1 'treesitter-mcp'")
    assert_match "treesitter-mcp", output
  end
end
