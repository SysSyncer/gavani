# Homebrew formula draft.
#
# Publish via a personal tap:
#   1. Push a v0.1.0 release (CI produces the tar.gz assets).
#   2. Create repo github.com/SysSyncer/homebrew-tap
#   3. Copy this file to Formula/gavani.rb in that repo.
#   4. Compute the sha256 of each release asset:
#        sha256sum gavani-0.1.0-*.tar.gz
#      and paste them below.
#   5. Users install with:
#        brew install SysSyncer/tap/gavani
#
# Once the project is popular enough it can be moved to homebrew-core by
# opening a PR there (requirements: notable popularity, test block, stable URL).

class Gavani < Formula
  desc "Keyboard-driven focus stopwatch TUI"
  homepage "https://github.com/SysSyncer/gavani"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/SysSyncer/gavani/releases/download/v0.1.0/gavani-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64_MAC_SHA256"
    else
      url "https://github.com/SysSyncer/gavani/releases/download/v0.1.0/gavani-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_INTEL_MAC_SHA256"
    end
  end

  on_linux do
    url "https://github.com/SysSyncer/gavani/releases/download/v0.1.0/gavani-0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_LINUX_SHA256"
  end

  def install
    bin.install "gavani"
  end
end
