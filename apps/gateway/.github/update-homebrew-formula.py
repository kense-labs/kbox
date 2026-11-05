#!/usr/bin/env python3
"""Regenerate Formula/krun-gateway.rb with new version and sha256 values."""
import os
import sys

ver             = os.environ["VER"]
sha_macos_arm64 = os.environ["SHA_MACOS_ARM64"]
sha_macos_x64   = os.environ["SHA_MACOS_X64"]
sha_linux_arm64 = os.environ["SHA_LINUX_ARM64"]
sha_linux_x64   = os.environ["SHA_LINUX_X64"]

base = f"https://github.com/kense-labs/krun/releases/download/v{ver}"

formula = (
    'class KenseGateway < Formula\n'
    '  desc "AI-native API gateway — SSE streaming, scale-to-zero, safe model rollouts"\n'
    '  homepage "https://github.com/kense-labs/krun"\n'
    f'  version "{ver}"\n'
    '  license "MIT"\n'
    '\n'
    '  on_macos do\n'
    '    on_arm do\n'
    f'      url "{base}/krun-gateway-{ver}-darwin-arm64.tar.gz"\n'
    f'      sha256 "{sha_macos_arm64}"\n'
    '    end\n'
    '    on_intel do\n'
    f'      url "{base}/krun-gateway-{ver}-darwin-x86_64.tar.gz"\n'
    f'      sha256 "{sha_macos_x64}"\n'
    '    end\n'
    '  end\n'
    '\n'
    '  on_linux do\n'
    '    on_arm do\n'
    f'      url "{base}/krun-gateway-{ver}-linux-arm64-musl.tar.gz"\n'
    f'      sha256 "{sha_linux_arm64}"\n'
    '    end\n'
    '    on_intel do\n'
    f'      url "{base}/krun-gateway-{ver}-linux-x86_64-musl.tar.gz"\n'
    f'      sha256 "{sha_linux_x64}"\n'
    '    end\n'
    '  end\n'
    '\n'
    '  def install\n'
    '    bin.install "krun-gateway"\n'
    '  end\n'
    '\n'
    '  test do\n'
    '    assert_match "krun-gateway", shell_output("#{bin}/krun-gateway --version")\n'
    '  end\n'
    'end\n'
)

out = "Formula/krun-gateway.rb"
with open(out, "w") as f:
    f.write(formula)
print(f"Written {out} for v{ver}")
