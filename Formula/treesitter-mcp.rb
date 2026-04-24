class TreesitterMcp < Formula
  desc "AST-first MCP server for token-efficient code analysis"
  homepage "https://github.com/Christoph/treesitter-mcp"
  version "0.6.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/Christoph/treesitter-mcp/releases/download/v0.6.0/treesitter-mcp-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/Christoph/treesitter-mcp/releases/download/v0.6.0/treesitter-mcp-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Christoph/treesitter-mcp/releases/download/v0.6.0/treesitter-mcp-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "treesitter-mcp"
  end

  test do
    assert_predicate bin/"treesitter-mcp", :exist?
    assert_predicate bin/"treesitter-mcp", :executable?
  end
end
