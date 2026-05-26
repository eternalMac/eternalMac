---
title: Installation
description: Install eternalMac from the public Homebrew tap.
---

Install from the public Homebrew tap:

```bash
brew install eternalmac/eternalmac/eternalmac
```

For local development, build from source:

```bash
git clone https://github.com/eternalMac/eternalMac
cd eternalMac
cargo build
```

`eternalMac` installs or verifies these runtime tools during setup:

- Eternal Terminal
- tmux
- Mutagen
- Tailscale
- launchd agents

The MVP is macOS-only and assumes Homebrew-managed dependencies.
