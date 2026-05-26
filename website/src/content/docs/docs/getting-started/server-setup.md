---
title: Server Setup
description: Configure the Mac mini as the eternalMac devserver.
---

Run this on the Mac mini:

```bash
eternalMac setup server
```

Server setup verifies Tailscale, checks Remote Login availability, starts the root Homebrew-managed Eternal Terminal service, creates the default tmux session, and installs a launchd agent for the server daemon.

macOS may ask for your password so eternalMac can run `sudo brew services start et`. Homebrew's Eternal Terminal service requires root.

At the end, copy the printed Tailscale DNS name. You will use it on the laptop.

```bash
Server DNS: mac-mini.example.ts.net
```

Remote Login must be enabled in macOS because Eternal Terminal and Mutagen both rely on SSH during setup and handshaking.
