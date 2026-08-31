# sddk.rb — Homebrew formula for the SDDK CLI.
#
# Published to the Rubentxu/homebrew-sddk tap. The release.yml pipeline
# builds the binaries; this formula points at the GitHub Release assets and
# Homebrew verifies each download against its published sha256.
#
# NOTE: darwin-x86_64 (Intel macOS) assets are published starting with the
# next tag after this formula's initial cut; Intel Macs get an explicit
# message until then.
class Sddk < Formula
  desc "Deterministic SDDK workflow tooling (spec-driven development kernel)"
  homepage "https://github.com/Rubentxu/software-development-decision-kernel"
  license "MIT"
  version "1.0.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rubentxu/software-development-decision-kernel/releases/download/v1.0.0/sddk-darwin-arm64"
      sha256 "ee5052aed50084213d0c32686f0a8f7fcec0673c6865c3e1c7ebae0fc5a8ed8e"
    else
      odie "sddk Intel macOS binaries are not published yet (next release). Use the installer: https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/Rubentxu/software-development-decision-kernel/releases/download/v1.0.0/sddk-linux-aarch64"
      sha256 "5fe97bd10e7e11f7ce376d00e13e5fe6ebc8558949fbd0d658cdc831410b5cb3"
    else
      url "https://github.com/Rubentxu/software-development-decision-kernel/releases/download/v1.0.0/sddk-linux-x86_64"
      sha256 "b4bd674cf2269787f901a949f953e773226f1b43c6df16b12f3c2236ab22e6c1"
    end
  end

  def install
    bin.install "sddk"
  end

  test do
    assert_match "sddk", shell_output("#{bin}/sddk --version")
  end
end
