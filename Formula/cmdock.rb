class Cmdock < Formula
  desc "CLI game for Linux and Docker command practice"
  homepage "https://github.com/torifo/cmd-mock-cli"
  license "MIT"
  head "https://github.com/torifo/cmd-mock-cli.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", ".", "--root", prefix
  end

  test do
    assert_match "cmdock", shell_output("#{bin}/cmdock --list")
  end
end
