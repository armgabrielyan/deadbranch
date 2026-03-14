# Homebrew Formula for deadbranch
#
# This formula is available via:
#   brew install armgabrielyan/tap/deadbranch
#
# Or build from source:
#   brew install --HEAD armgabrielyan/tap/deadbranch

class Deadbranch < Formula
  desc "Clean up stale git branches safely"
  homepage "https://github.com/armgabrielyan/deadbranch"
  license "MIT"
  version "0.3.0" # x-release-please-version

  # Binary releases for different platforms
  on_macos do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "3173c7b8f432ed2d28cbc2530a13199baf8b47a5e65beebdc165dbea1eb66459"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "dd49ee71df838f0b3e58cde378bd224898bba73e86dfe14c86c1c974dbc6a642"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "413f44edde9ef7d04cf9bcd799ffa0c1fcad61d3d243c405b496881993e3f931"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f1a03fde5134d0e4b30a9e66bd37183237788ccaea12234993d42d919c522055"
    end
  end

  # Build from source (for --HEAD installs)
  head do
    url "https://github.com/armgabrielyan/deadbranch.git", branch: "main"
    depends_on "rust" => :build
  end

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args
    else
      bin.install "deadbranch"
      man1.install "deadbranch.1"
    end
  end

  test do
    assert_match "deadbranch", shell_output("#{bin}/deadbranch --version")
  end
end
