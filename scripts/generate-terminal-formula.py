#!/usr/bin/env python3
"""Generate a Homebrew formula from the exact checksums of release archives."""
import argparse
import pathlib
import re

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('directory', type=pathlib.Path)
p.add_argument('--tag', required=True)
p.add_argument('--repository', default='Revaycolizer/overseer')
a = p.parse_args()
version = re.search(r'^version = "([^"]+)"', pathlib.Path('Cargo.toml').read_text(), re.M)[1]
if a.tag != 'v' + version or not re.fullmatch(r'[\w.-]+/[\w.-]+', a.repository):
    raise SystemExit('Tag must match Cargo.toml and repository must be owner/name.')
base = f'https://github.com/{a.repository}/releases/download/{a.tag}'

def asset(target, indent):
    name = f'overseer-terminal-{a.tag}-{target}.tar.gz'
    checksum, filename = (a.directory / (name + '.sha256')).read_text().strip().split('  ')
    if filename != name or not re.fullmatch('[0-9a-f]{64}', checksum):
        raise SystemExit('Invalid checksum file')
    return f'{indent}url "{base}/{name}"\n{indent}sha256 "{checksum}"\n'

formula = f'''class Overseer < Formula
  desc "Local-first AI assistant for the terminal"
  homepage "https://github.com/{a.repository}"
  version "{version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
'''+asset('aarch64-apple-darwin', '      ')+'''    else
'''+asset('x86_64-apple-darwin', '      ')+'''    end
  end

  on_linux do
    on_intel do
'''+asset('x86_64-unknown-linux-gnu', '      ')+'''    end
  end

  def install
    bin.install "overseer-tui"
  end

  def caveats
    "Run overseer-tui. Configure Ollama or a cloud provider in Settings. Linux requires Ubuntu 24.04-compatible libraries and libasound2."
  end

  test do
    assert_equal "overseer-tui #{version}", shell_output("#{bin}/overseer-tui --version").strip
  end
end
'''
(a.directory / 'overseer.rb').write_text(formula)
print(a.directory / 'overseer.rb')
