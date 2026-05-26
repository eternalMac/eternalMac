# Homebrew Packaging

`eternalMac` ships through a custom Homebrew tap first. The formula is generated from `eternalmac.rb.tmpl` and expects a release tarball containing the compiled `eternalMac` binary.

## Local Formula Test

From the repository root:

```bash
scripts/release/package-homebrew.sh
scripts/release/install-homebrew-local.sh
```

The default generated formula uses a `file://` URL pointing at the local tarball in `target/homebrew`. Homebrew 5 requires formulae to live inside a tap, so the local installer copies the generated formula into a local `eternalmac/eternalmac` tap before installing it.

Generated tarballs include the release binary plus root `LICENSE` and `NOTICE` files. The formula installs those metadata files with the package.

## Release Formula

For a GitHub Release, build the package and stamp the final download URL into the formula:

```bash
scripts/release/package-homebrew.sh \
  --version 0.1.2 \
  --url https://github.com/eternalMac/eternalMac/releases/download/v0.1.2/eternalmac-0.1.2-aarch64-apple-darwin.tar.gz \
  --formula-output Formula/eternalmac.rb
```

Copy the generated `Formula/eternalmac.rb` into the tap repository, commit it, and push the tap.

Public install command:

```bash
brew install eternalmac/eternalmac/eternalmac
```

That tap name maps to a GitHub repository named `eternalMac/homebrew-eternalmac`.

Audit before publishing:

```bash
brew audit --strict --online eternalmac/eternalmac/eternalmac
```

The audit requires public unauthenticated access to the formula homepage URL and release asset URL.

## Notes

- The generated formula is architecture-specific because it installs a compiled binary tarball.
- The formula installs `et`, `tmux`, and Mutagen as runtime dependencies.
- Tailscale remains setup-managed because it is installed as a cask and may require user interaction.
