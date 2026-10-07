#!/usr/bin/env python3
"""Generate a Homebrew formula from the exact checksums of release archives."""
import argparse
import pathlib
import re

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('directory', type=pathlib.Path)
p.add_argument('--tag', required=True)
p.add_argument('--repository', default='Revaycolizer/overseer')
p.add_argument('--target', choices=['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu'], action='append', help='Include only verified targets; default includes all three')
a = p.parse_args()
version = re.search(r'^version = "([^"]+)"', pathlib.Path('Cargo.toml').read_text(), re.M)[1]
if not re.fullmatch(r'(?:terminal-)?v' + re.escape(version) + r'(?:-preview\.[0-9]+)?', a.tag) or not re.fullmatch(r'[\w.-]+/[\w.-]+', a.repository):
    raise SystemExit('Tag must match Cargo.toml and repository must be owner/name.')
base = f'https://github.com/{a.repository}/releases/download/{a.tag}'

def asset(target, indent):
    name = f'overseer-terminal-v{version}-{target}.tar.gz'
    checksum, filename = (a.directory / (name + '.sha256')).read_text().strip().split('  ')
    if filename != name or not re.fullmatch('[0-9a-f]{64}', checksum):
        raise SystemExit('Invalid checksum file')
    return f'{indent}url "{base}/{name}"\n{indent}sha256 "{checksum}"\n'

targets = a.target or ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']
formula = f'''class Overseer < Formula
  desc "Local-first AI assistant for the terminal"
  homepage "https://github.com/{a.repository}"
  version "{version}"
  license "MIT"
'''
mac = [t for t in targets if 'apple' in t]
if mac:
    formula += "\n  on_macos do\n"
    for target in mac:
        architecture = 'on_arm' if target.startswith('aarch64') else 'on_intel'
        formula += f"    {architecture} do\n" + asset(target, '      ') + "    end\n"
    formula += "  end\n"
if 'x86_64-unknown-linux-gnu' in targets:
    formula += "\n  on_linux do\n    on_intel do\n" + asset('x86_64-unknown-linux-gnu', '      ') + "    end\n  end\n"
formula += '''
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
