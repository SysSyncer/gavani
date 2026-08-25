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
      sha256 "96a3ad1ebcdac9930d2d1827f9248f8f51b30df3df96dcc9a49c3926ef9ad533"
    else
      url "https://github.com/SysSyncer/gavani/releases/download/v0.1.0/gavani-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "4126eb67a1e1d0e3ca7c67cf19a6724ae317ac50d41d0449a21ad14cf4357754"
    end
  end

  on_linux do
    url "https://github.com/SysSyncer/gavani/releases/download/v0.1.0/gavani-0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "7f0e78beece6999dd06231e3419d9df5c9ff708be83b68cefff652486f32880c"
  end

  def install
    bin.install "gavani"
  end
end
