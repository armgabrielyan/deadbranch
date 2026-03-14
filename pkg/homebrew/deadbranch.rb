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
  version "0.2.1" # x-release-please-version

  # Binary releases for different platforms
  on_macos do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "86feb7e0382277a292e40ffeff08d79544bb8f469645df75b69c3dd503b6d864"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "06d9242a5a01e0935f2de9fe9801ca3503951d3f8638a543e67af9ccaf8e3e5c"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "d6168a1d49f6523caa1438d9b419ae1ad344acb2b91d299d77b8da427ba2ee86"
    end

    on_arm do
      url "https://github.com/armgabrielyan/deadbranch/releases/download/v#{version}/deadbranch-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "746529f77b0ceb7a7a011d151dae31565baf4b558e45cbd6cb1702661b418daa"
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
