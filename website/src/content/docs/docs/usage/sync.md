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

Project sync copies the selected tree as requested. If a specific project needs exclusions, pass one or more Mutagen ignore patterns:

```bash
eternalMac sync add project \
  --local ~/project \
  --remote ~/project \
  --ignore .env \
  --ignore "secrets/"
```

List configured sync pairs:

```bash
eternalMac sync list
```

Inspect Mutagen sync state:

```bash
eternalMac sync status
```

The MVP uses Mutagen two-way resolved sync. Conflict handling is intentionally simple for the first release.
