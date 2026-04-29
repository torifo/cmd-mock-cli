class Cmdock < Formula
  desc "CLI game for Linux and Docker command practice"
  homepage "https://github.com/torifo/cmd-mock-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/torifo/cmd-mock-cli/releases/download/v#{version}/cmdock-macos-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_AARCH64"
    else
      url "https://github.com/torifo/cmd-mock-cli/releases/download/v#{version}/cmdock-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  on_linux do
    url "https://github.com/torifo/cmd-mock-cli/releases/download/v#{version}/cmdock-linux-x86_64.tar.gz"
    sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
  end

  def install
    bin.install "cmdock"
  end

  test do
    assert_match "cmdock", shell_output("#{bin}/cmdock --list")
  end
end
