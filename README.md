# portmap

Local domain-to-port mapper. Routes custom `.localhost` domains to dev server ports via a reverse proxy, giving each project its own browser origin (separate cookies, localStorage, sessions).

## Why

When running multiple dev servers on `localhost:3000`, `localhost:8080`, etc., browsers treat them as the same origin. Cookies and sessions bleed across projects. `portmap` fixes this by letting you visit `http://my-project.localhost` instead — each domain gets its own cookie jar.

## How it works

1. You add a mapping: `my-project` → port `3000`
2. `portmap` adds `127.0.0.1 my-project.localhost` to `/etc/hosts`
3. A reverse proxy on port 80 routes requests by `Host` header to `127.0.0.1:3000`
4. Visit `http://my-project.localhost` in your browser
5. On exit, `/etc/hosts` is cleaned up automatically

Uses `.localhost` (RFC 6761) instead of `.local` to avoid macOS mDNS/Bonjour 5-second DNS delays.

## Install

```
curl -fsSL https://raw.githubusercontent.com/filiphric/portmap/main/install.sh | sh
```

This downloads the latest release binary for your Mac (Apple Silicon or Intel) and places it in `/usr/local/bin`.

### Build from source

If you prefer to build from source (requires [Rust](https://rustup.rs)):

```
cargo install --git https://github.com/filiphric/portmap
```

## Usage

Mappings are session-only — they're cleaned up when the tool stops. If not already root, `portmap` automatically re-runs itself under `sudo` and prompts for your password.

```
portmap
```

This launches a TUI where you manage your mappings:

```
┌─ portmap ──────────────────────── [a]dd [d]el [q]uit ─┐
│ Domain                │ Port   │ Status               │
│───────────────────────┼────────┼──────────────────────│
│▸ my-project.localhost │ 3000   │ ● Active             │
│  api.localhost        │ 8080   │ ● Active             │
│  dashboard.localhost  │ 5173   │ ● Port Unreachable   │
├───────────────────────┴────────┴──────────────────────┤
│ Proxy running on :80 │ 3 mappings                     │
└───────────────────────────────────────────────────────┘
```

### Keybindings

| Key | Action |
|-----|--------|
| `a` | Add a new mapping |
| `d` | Delete selected mapping |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `q` | Quit and clean up |

In the add-mapping popup:

| Key | Action |
|-----|--------|
| `Tab` | Switch between domain/port fields |
| `Enter` | Submit |
| `Esc` | Cancel |

Typing `my-project` in the domain field automatically maps to `my-project.localhost`.

### Cleanup

If `portmap` is killed with `SIGKILL` or during a power loss, leftover `/etc/hosts` entries can be removed with:

```
portmap --cleanup
```

Under normal circumstances (quitting with `q`, Ctrl+C, SIGTERM, or even a panic), cleanup happens automatically.
