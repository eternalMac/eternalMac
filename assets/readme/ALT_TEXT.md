# Social asset alt text

The SVG files in this directory are the editable sources. The same-named PNG
files and their copies in `website/public/socialAssets/` are generated assets.

After changing an SVG, regenerate and commit both PNG locations:

```bash
npm --prefix website run assets:write
```

CI runs `npm --prefix website run assets:check` and rejects stale, missing, or
invalid generated assets. The EternalMac website is the canonical social host:
`https://eternalmac.dev/socialAssets/architecture.png` and
`https://eternalmac.dev/socialAssets/terminal-proof.png`.

## architecture.png

Diagram showing a MacBook thin client connecting through Tailscale to an always-on Mac mini running Eternal Terminal, tmux, Mutagen, and launchd.

## terminal-proof.png

Terminal demonstration of eternalMac attach reconnecting to an overnight agent session on a Mac mini after a laptop wakes on a different network.
