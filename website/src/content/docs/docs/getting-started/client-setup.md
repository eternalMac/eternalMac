---
title: Client Setup
description: Configure the laptop that connects to the Mac mini.
---

Run this on the laptop:

```bash
eternalMac setup client --server <mac-mini-tailscale-dns>
```

Client setup asks for:

- the Mac mini Tailscale DNS name
- the server SSH username
- one or more sync roots

It creates a dedicated passwordless SSH key for `eternalMac`, authorizes it with a one-time password prompt when needed, discovers the remote `etterminal` path, creates Mutagen sync sessions, and installs the client daemon.

After setup:

```bash
eternalMac status
eternalMac doctor
```
