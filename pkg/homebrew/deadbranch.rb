# Homebrew Formula for deadbranch
#
# This formula is available via:
#   brew tap armgabrielyan/deadbranch https://github.com/armgabrielyan/homebrew-deadbranch
#   brew install deadbranch
#
# Or build from source:
#   brew install --HEAD armgabrielyan/deadbranch/deadbranch

class Deadbranch < Formula
  desc "Clean up stale git branches safely"
  homepage "https://github.com/armgabrielyan/deadbranch"
  license "MIT"
  version "0.1.1" # x-release-please-version

  # Binary releases for different platforms
  on_macos do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-apple-darwin.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-apple-darwin.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
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
    end
  end

  test do
    assert_match "deadbranch", shell_output("#{bin}/deadbranch --version")
  end
end
