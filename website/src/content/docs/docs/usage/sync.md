---
title: Sync
description: Add and inspect project folder sync pairs.
---

Add a sync pair:

```bash
eternalMac sync add project \
  --local ~/project \
  --remote ~/project
```

Bare remote paths are resolved against the paired Mac mini recorded during client setup. You can still pass a full Mutagen endpoint, such as `devuser@mac-mini.example.ts.net:~/project`, when you need to override the paired server.

List configured sync pairs:

```bash
eternalMac sync list
```

Inspect Mutagen sync state:

```bash
eternalMac sync status
```

The MVP uses Mutagen two-way resolved sync. Conflict handling is intentionally simple for the first release.
