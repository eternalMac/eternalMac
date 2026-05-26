# eternalMac

`eternalMac` turns a Mac Mini into a personal devserver for a laptop.

The current MVP is a macOS-only Rust CLI that wraps:

- Eternal Terminal for resilient remote shell access
- `tmux` for named remote sessions
- Mutagen for file sync
- Tailscale for private reachability
- `launchd` for always-on background operation

Product docs: https://eternalmac.dev/docs

## Current Scope

Today, the repo provides:

- `eternalMac setup server` to configure a Mac Mini as the devserver
- `eternalMac setup client` to configure a laptop as the thin client
- `eternalMac attach [session]` to connect to a named remote `tmux` session
- `eternalMac attach -n <session>` to create a new remote `tmux` session and attach to it
- `eternalMac session ...` to list, create, pin, and unpin sessions
- `eternalMac sync ...` to add and inspect sync pairs
- `eternalMac status` and `eternalMac doctor` for local health and setup checks

The tool currently assumes Homebrew-managed dependencies and installs or checks:

- `et`
- `tmux`
- `mutagen`
- `tailscale-app`

## Platform

- macOS only
- Homebrew-first workflow
- Single-user personal devserver model

The Mac Mini must have Remote Login enabled because Eternal Terminal and Mutagen both rely on SSH for setup and handshaking. Server setup may ask for your macOS password to start Homebrew's root Eternal Terminal service. Client setup asks for the server SSH username, creates a dedicated passwordless `eternalMac` SSH key, authorizes it with a one-time password prompt when needed, and records the remote `etterminal` path used by ET.

## Quick Start

Install from the eternalMac Homebrew tap:

```bash
brew install eternalmac/eternalmac/eternalmac
```

On the Mac Mini:

```bash
eternalMac setup server
```

On the laptop:

```bash
eternalMac setup client --server <tailscale-dns-name>
```

Then attach:

```bash
eternalMac attach
```

Create a fresh remote session and attach immediately:

```bash
eternalMac attach -n feature-branch
```

Full setup and troubleshooting docs:

https://eternalmac.dev/docs

## Command Surface

```bash
eternalMac setup server
eternalMac setup client [--server <dns-name>]

eternalMac attach [session]
eternalMac attach -n <session>

eternalMac session list
eternalMac session new <name>
eternalMac session pin <name>
eternalMac session unpin <name>

eternalMac sync add <name> --local <path> --remote <path>
eternalMac sync list
eternalMac sync status

eternalMac status
eternalMac doctor
```

`eternalMac doctor` exits non-zero when it prints issues, making it suitable for release smoke checks and automation gates.

## Development

Build:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

Run the smoke check:

```bash
bash scripts/smoke/bootstrap.sh
```

## Packaging

The repo includes Homebrew packaging support under `packaging/homebrew`. Local release packaging can be validated with:

```bash
scripts/release/package-homebrew.sh
scripts/release/install-homebrew-local.sh
```

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

eternalMac is licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
