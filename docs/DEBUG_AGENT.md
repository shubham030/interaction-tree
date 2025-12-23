# Debug Agent Architecture

A fully autonomous Flutter runtime expert that handles sessions, app lifecycle, and runtime investigation.

## Overview

The Debug Agent follows a **strict separation of concerns**:
- **Debug Agent** investigates the RUNNING APP only (no code access)
- **Amp** reads source code and applies fixes based on the agent's report

```
┌─────────────────────────────────────────────────────────────────┐
│   DEBUG AGENT (Runtime Expert)    │    AMP (Code Expert)       │
│                                   │                            │
│   • Investigate running app       │    • Read debug reports    │
│   • Analyze logs, errors          │    • Read source code      │
│   • Inspect widget tree/state     │    • Apply code fixes      │
│   • Execute interactions          │    • Run tests, analyze    │
│   • Curate debug reports          │    • Hot reload/restart    │
│                                   │                            │
│   Tools: Fleeter MCP only         │    Tools: Read, Grep,      │
│   (NO code access)                │    edit_file, Bash, etc.   │
└─────────────────────────────────────────────────────────────────┘
```

## Invocation

Amp invokes the Debug Agent via the `send_debug_message` MCP tool:

```typescript
send_debug_message({
  message: "Debug why the cart doesn't update when removing items",
  projectPath: "/path/to/flutter/project"  // Required
})
```

## Debug Agent Capabilities

### Available Tools (Fleeter MCP)

| Category | Tools |
|----------|-------|
| **Session Management** | `listSessions`, `createSession`, `connectSession`, `destroySession` |
| **App Lifecycle** | `runApp`, `stopApp`, `hotReload`, `hotRestart`, `getStatus` |
| **Runtime Investigation** | `getTree`, `execute`, `getState`, `batch`, `getLogs`, `getErrors` |

### Blocked Tools (Amp's Responsibility)

- `Read`, `Grep`, `glob`, `finder` - Amp reads source code
- `Bash` - Amp runs flutter analyze, tests, etc.
- `edit_file`, `create_file` - Amp applies code fixes

## Workflows

### Investigation Workflow

```
1. SETUP: Ensure session exists and app is running
   └── listSessions → createSession/connectSession → runApp

2. INVESTIGATE: Gather runtime data
   └── getTree, getLogs, getErrors, getState

3. REPRODUCE: Execute interactions to trigger the issue
   └── execute, batch

4. HYPOTHESIZE: Form hypothesis based on observations

5. REPORT: Return structured debug report
```

### Validation Workflow (Post-Fix)

```
1. Determine reload type: UI-only → hotReload, State/logic → hotRestart
2. Track current navigation state
3. Apply changes via hotReload or hotRestart
4. Restore navigation if hotRestart was used
5. Test the fix with reproduction steps
6. Return validation report
```

## Report Formats

### Debug Report

```markdown
## 🔍 Debug Report: [Issue Title]

### Summary
One-line summary of runtime observations.

### Runtime Observations
- **Symptoms**: What was observed in the running app
- **Errors in Logs**: Error messages with timestamps
- **Widget Tree**: Relevant widgets and structure
- **Widget States**: State snapshots

### Reproduction Steps
| Step | Action | Result | Observations |
|------|--------|--------|--------------|
| 1 | execute("btn", "tap") | ✓/✗ | What happened |

### Runtime Hypothesis
Analysis based on runtime behavior (NOT code).

### Suggested Investigation (for Amp)
- Keywords to search in codebase
- What code patterns to look for
```

### Validation Report

```markdown
## ✅ Validation Report: [Fix Title]

### Change Type & Reload
- Change type: UI-only / State-logic
- Reload method: hotReload / hotRestart

### Test Steps
| Step | Action | Expected | Actual | Status |
|------|--------|----------|--------|--------|

### Result
**✓ FIXED** or **✗ NOT FIXED** with details
```

## Implementation Details

### Files

- `packages/fleeter-daemon/src/agent/executor.ts` - Debug agent executor
- `packages/fleeter-daemon/src/agent/prompts.ts` - System prompts
- `packages/fleeter-mcp-proxy/src/proxy.ts` - MCP proxy (single tool: send_debug_message)
- `.agents/debug-agent.md` - Agent definition for Amp

### Timeouts

- `debug_agent_message`: 10 minutes (agent runs Claude Code, multiple tool calls)
- `run_app`: 5 minutes (cold builds can be slow)
- Default commands: 30 seconds

### Model

Default model: `claude-haiku-4-5-20251001`
