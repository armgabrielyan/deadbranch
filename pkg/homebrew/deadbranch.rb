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
  version "0.2.0" # x-release-please-version

  # Binary releases for different platforms
  on_macos do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "b1d4c399a9ba4cfb3e348376660077052d9abf74c7a4a4fef52066fa1637a0c0"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "201d2921a90bd6f8c7bde909a001ee5496a6a98b43df931800f91905d961437f"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a5b8db6337edebe1a2abb4aec4cada9854cae7a2d2351babf2f992499ddac781"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "94934f8e0c382be120b3e7fca7b07ffc5719774d450d9b6e33037b492aaead72"
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
