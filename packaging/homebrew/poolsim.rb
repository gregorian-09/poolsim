class Poolsim < Formula
  desc "Connection pool sizing simulator"
  homepage "https://github.com/gregorian-09/poolsim"
  url "https://github.com/gregorian-09/poolsim/archive/refs/tags/v0.2.1.tar.gz"
  sha256 "53be7c5f314437be2654f5306412ad1a4a858a2ad848e381be82dc483701fdaa"
  license "MIT"
  head "https://github.com/gregorian-09/poolsim.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", "crates/poolsim-cli", "--root", prefix
    system "cargo", "install", "--locked", "--path", "crates/poolsim-web", "--root", prefix
  end

  test do
    assert_match "Connection pool sizing simulator", shell_output("#{bin}/poolsim --help")
    assert_match "poolsim-web", shell_output("#{bin}/poolsim-web --help", 1)
  end
end
