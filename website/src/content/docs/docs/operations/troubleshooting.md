---
title: Troubleshooting
description: Common setup and runtime issues.
---

## Tailscale is not connected

Open Tailscale, sign in, and rerun setup.

## SSH is unreachable

Enable Remote Login on the Mac mini:

```text
System Settings -> General -> Sharing -> Remote Login
```

## ET cannot find etterminal

Rerun client setup. It discovers and stores the remote `etterminal` path used by Eternal Terminal.

```bash
eternalMac setup client --server <mac-mini-tailscale-dns>
```

## ET server is unreachable

Run this on the Mac mini:

```bash
sudo brew services list | grep et
sudo brew services start et
nc -G 5 -z localhost 2022
```

Then rerun server setup if the service was stopped:

```bash
eternalMac setup server
```

## Sync is degraded

Run:

```bash
eternalMac sync status
eternalMac doctor
```

If duplicate Mutagen sessions exist, manual cleanup may be required until first-class sync removal lands.
