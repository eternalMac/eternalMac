---
title: Installation
description: Install eternalMac from source while the Homebrew tap is being prepared.
---

The first public release will use the project Homebrew tap:

```bash
brew install eternalmac/eternalmac/eternalmac
```

Until that release is cut, run from source:

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
