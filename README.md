# eternalMac

`eternalMac` turns a Mac Mini into a personal devserver for a laptop.

The current MVP is a macOS-only Rust CLI that wraps:

- Eternal Terminal for resilient remote shell access
- `tmux` for named remote sessions
- Mutagen for file sync
- Tailscale for private reachability
- `launchd` for always-on background operation

Project documentation is intentionally lean for now. Longer-form product docs will move to the project website later.

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

The Mac Mini must have Remote Login enabled because Eternal Terminal and Mutagen both rely on SSH for setup and handshaking. Client setup asks for the server SSH username, creates a dedicated passwordless `eternalMac` SSH key, authorizes it with a one-time password prompt when needed, and records the remote `etterminal` path used by ET.

## Quick Start

Source build:

```bash
cargo build
```

On the Mac Mini:

```bash
cargo run -- setup server
```

On the laptop:

```bash
cargo run -- setup client --server <tailscale-dns-name>
```

Then attach:

```bash
cargo run -- attach
```

Create a fresh remote session and attach immediately:

```bash
cargo run -- attach -n feature-branch
```

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

The repo includes a Homebrew formula template at `packaging/homebrew/eternalmac.rb.tmpl`.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).
