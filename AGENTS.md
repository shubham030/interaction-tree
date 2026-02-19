# Interaction Tree / Fleeter

A Flutter testing and AI-driven interaction system with a central daemon architecture.

## Repository Structure

```
interaction_tree/
├── lib/                          # Core Flutter package (InteractionKey, InteractableMixin)
├── packages/
│   ├── fleeter-daemon/           # Central daemon (TypeScript/Bun) - manages sessions, Flutter processes
│   ├── fleeter-mcp-proxy/        # Thin MCP proxy for LLM clients (TypeScript/Bun)
│   ├── interaction_tree_driver/  # Flutter Driver integration (Dart)
│   ├── interaction_tree_mcp/     # Legacy MCP server (Dart) - deprecated
│   └── interaction-tree-server/  # Legacy server (TypeScript) - deprecated
├── tui/                          # Rust TUI for monitoring sessions
├── example/                      # Example Flutter app with interaction annotations
└── test/                         # Dart package tests
```

## Architecture

See [DAEMON_DESIGN.md](./DAEMON_DESIGN.md) for full architecture documentation.

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Amp Thread 1   │     │  Amp Thread 2   │     │      TUI        │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │ stdio                 │ stdio                 │
         ▼                       ▼                       │
┌─────────────────┐     ┌─────────────────┐              │
│  MCP Proxy #1   │     │  MCP Proxy #2   │              │
└────────┬────────┘     └────────┬────────┘              │
         │ ws://                 │ ws://                 │ ws://
         └───────────────────────┼───────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │    FLEETER DAEMON      │
                    │    (single process)    │
                    │  ┌──────────────────┐  │
                    │  │  SessionManager  │  │
                    │  │  FlutterProcess  │  │
                    │  │  AI Agent        │  │
                    │  └──────────────────┘  │
                    │  WebSocket :9877       │
                    └────────────────────────┘
```

## Commands

```bash
# Build & install
bun install
bun run build

# Run daemon directly (preferred for development)
node packages/fleeter-daemon/dist/bin/daemon.js          # Minimal logs (info/warn/error)
node packages/fleeter-daemon/dist/bin/daemon.js -v       # Verbose logs (debug)

# Daemon as launchd service (optional)
bun run daemon:install      # Install & start
bun run daemon:uninstall    # Stop & remove
bun run daemon:status       # Check status

# Logs (~/.fleeter/logs/)
bun run logs:daemon         # Tail daemon logs
bun run logs:clear          # Clear all logs

# Development
bun run build               # Build TypeScript packages
bun run test                # Run tests (vitest)
bun run typecheck           # Type check
bun run lint                # Lint

# Flutter package
dart test                   # Dart tests
dart analyze                # Analyze

# TUI (Rust)
cd tui && cargo run
```

## Key Files

| File | Purpose |
|------|---------|
| `packages/fleeter-daemon/src/daemon.ts` | Main daemon class |
| `packages/fleeter-daemon/src/agent/executor.ts` | AI agent execution (Claude Agent SDK) |
| `packages/fleeter-daemon/src/flutter/process-manager.ts` | Flutter process lifecycle |
| `packages/fleeter-daemon/src/session/manager.ts` | Session management |
| `packages/fleeter-mcp-proxy/src/proxy.ts` | MCP protocol translation |
| `lib/src/interaction_key.dart` | Core InteractionKey widget |
| `tui/src/main.rs` | TUI entry point |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FLEETER_PORT` | `9877` | WebSocket server port |
| `FLEETER_HOST` | `127.0.0.1` | Host to bind to |

## MCP Configuration

Add to `~/.config/amp/settings.json`:

```json
{
  "mcpServers": {
    "dart": {
      "command": "bun",
      "args": ["run", "--cwd", "/path/to/interaction_tree/packages/fleeter-mcp-proxy", "start"]
    }
  }
}
```

## Code Style

- **Dart**: `flutter_lints`
- **TypeScript**: ESLint, no semicolons
- **Rust**: rustfmt
