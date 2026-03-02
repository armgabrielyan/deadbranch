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
  version "0.1.4" # x-release-please-version

  # Binary releases for different platforms
  on_macos do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "669b7c75ccdb3a841b98bb6c265a82ea4ce26281dfcbe6080cbbe892ff96e9ef"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "f770bbd396a23ed8e53a324f8f821a0c24cc0ff87abe4acd632e5367ad094e2d"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "3ea8207b52aafc498923beeefcc6607759bae4e7edd0cad5048a98343e117228"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "56e50bcbe634ef85dd81e1da01cb681fb7e1e661236998515565c32216beed9a"
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
