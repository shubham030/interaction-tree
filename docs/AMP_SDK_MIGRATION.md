# Amp SDK Migration

This document describes the migration from `@anthropic-ai/claude-agent-sdk` to `@sourcegraph/amp-sdk` for the Fleeter debug agent.

## Overview

The debug agent now uses **Amp SDK** instead of Claude Agent SDK. Key changes:

1. **Rush Mode** - Uses Amp's `rush` mode for 67% cheaper and 50% faster execution
2. **Thread-based Debugging** - Each debug session creates an Amp thread that can be handed off to the main Amp instance
3. **Simplified Architecture** - No need to find Claude Code executable; Amp SDK handles everything

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        MAIN AMP INSTANCE                            │
│                    (User's editor/terminal)                         │
│                                                                     │
│   User: "Debug why the cart doesn't update"                         │
│   Amp: Calls fleeter MCP → send_debug_message                      │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      FLEETER DAEMON                                 │
│                                                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │              DEBUG AGENT (Amp SDK - Rush Mode)              │   │
│   │                                                             │   │
│   │  • Runs in rush mode (fast, cheap)                          │   │
│   │  • Creates Amp thread: T-xxx-xxx-xxx                        │   │
│   │  • Investigates runtime (logs, errors, widget tree)         │   │
│   │  • Returns thread-id + report to main Amp                   │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│   Result: {                                                         │
│     status: "success",                                              │
│     summary: "Bug analysis report...",                              │
│     threadId: "T-abc123-def456",                                    │
│     threadUrl: "https://ampcode.com/threads/T-abc123-def456"        │
│   }                                                                 │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        MAIN AMP INSTANCE                            │
│                                                                     │
│   Amp receives report + thread-id                                   │
│   Amp: "I found the issue. The debug thread is available at:        │
│         https://ampcode.com/threads/T-abc123-def456                 │
│         Let me fix the code based on the findings..."               │
│                                                                     │
│   - Reads source code                                               │
│   - Applies fix                                                     │
│   - Asks debug agent to validate (continues same thread)            │
└─────────────────────────────────────────────────────────────────────┘
```

## Thread Handoff Flow

1. **Main Amp** receives a bug report from user
2. **Main Amp** calls the `send_debug_message` MCP tool
3. **Fleeter Daemon** spawns a debug agent using Amp SDK in rush mode
4. **Debug Agent** investigates the running app (logs, errors, widget tree)
5. **Debug Agent** returns:
   - `summary`: The debug report with observations and hypotheses
   - `threadId`: Amp thread ID (e.g., `T-abc123-def456`)
   - `threadUrl`: Full URL for reference
6. **Main Amp** can:
   - Reference the thread URL in its response
   - Read source code based on keywords in the report
   - Apply fixes
   - Continue the debug thread for validation

## Configuration

### Agent Modes

The debug agent supports two modes:

| Mode | Speed | Cost | Use Case |
|------|-------|------|----------|
| `rush` (default) | 50% faster | 67% cheaper | Information extraction, simple bugs |
| `smart` | Slower | Full price | Complex bugs requiring deep analysis |

```typescript
const executor = new DebugAgentExecutor({ 
  mode: 'rush'  // default - fast & cheap
});

// Switch modes dynamically
executor.setMode('smart');  // For complex bugs

// Or per-execution
await executor.execute({ 
  intent: "Complex state bug", 
  mode: 'smart',  // Override for this call only
  cwd 
});
```

### Thread Privacy

Threads are **private by default** for security. Make them public when creating PRs:

```typescript
const executor = new DebugAgentExecutor({
  visibility: 'private'  // default
});

// After investigation, make public for PR
const publicUrl = executor.makePublic();
console.log(publicUrl); // https://ampcode.com/threads/T-abc123-def456

// Or set visibility per-execution
await executor.execute({
  intent: "Debug for PR",
  visibility: 'public',  // This session will be public
  cwd
});
```

### Thread Tagging

Tag threads for easy filtering and categorization:

```typescript
const executor = new DebugAgentExecutor({
  tags: {
    project: 'tutero-app',
    category: 'cart',
    issueRef: 'TUTERO-123',
    custom: ['regression', 'urgent']
  }
});

// Add tags dynamically
executor.addTag('category', 'checkout');
executor.addTag('custom', 'flaky-test');

// Or per-execution
await executor.execute({
  intent: "Debug checkout",
  tags: { category: 'checkout', issueRef: 'TUTERO-456' },
  cwd
});

// Get formatted tags
console.log(executor.getTags());
// { project: 'tutero-app', category: 'checkout', issueRef: 'TUTERO-456', custom: ['regression', 'urgent', 'flaky-test'] }
```

### Multi-turn Debugging

The debug agent maintains thread continuity automatically:

```typescript
const executor = new DebugAgentExecutor();

// First investigation
const result1 = await executor.execute({ intent: "Debug cart issue", cwd });
console.log(result1.threadId); // T-abc123-def456
console.log(result1.visibility); // 'private'
console.log(result1.tags); // { category: 'cart' }

// Continue on same thread
const result2 = await executor.execute({ intent: "Now check the state", cwd });
// Uses same threadId automatically

// Get thread URL for handoff
console.log(executor.getThreadUrl()); // https://ampcode.com/threads/T-abc123-def456

// Start fresh investigation
executor.clearThread();
```

### MCP Configuration

The debug agent uses `mcpConfig` to connect to fleeter tools:

```typescript
const mcpConfig: MCPConfig = {
  fleeter: {
    command: 'bun',
    args: ['run', '--cwd', '/path/to/fleeter-mcp-proxy', 'start'],
    env: {
      FLEETER_PROJECT_PATH: '/path/to/flutter/project',
    },
  },
};

await executor.execute({
  intent: "Debug issue",
  cwd,
  mcpConfig,  // Custom MCP config
});
```

## API Changes

### Before (Claude Agent SDK)

```typescript
import { query, Options } from '@anthropic-ai/claude-agent-sdk';

const options: Options = {
  model: 'claude-haiku-4-5-20251001',
  systemPrompt: SYSTEM_PROMPT,
  allowDangerouslySkipPermissions: true,
};

for await (const message of query({ prompt, options })) {
  if (message.type === 'system' && message.subtype === 'init') {
    sessionId = message.session_id;
  }
}
```

### After (Amp SDK)

```typescript
import { execute, createPermission } from '@sourcegraph/amp-sdk';

const options = {
  cwd: projectPath,
  dangerouslyAllowAll: true,
  permissions: [
    createPermission('mcp__fleeter__*', 'allow'),
    createPermission('Read', 'reject'),
  ],
};

for await (const message of execute({ prompt, options })) {
  if (message.type === 'system' && message.subtype === 'init') {
    threadId = message.thread_id;
  }
}
```

## Key Differences

| Feature | Claude Agent SDK | Amp SDK |
|---------|------------------|---------|
| Model selection | Manual (`claude-haiku-4-5`) | Automatic based on mode |
| Mode | N/A | `rush` / `smart` / `free` |
| Session ID | `session_id` | `thread_id` |
| Resumption | `options.resume = sessionId` | `options.continue = threadId` |
| Thread URL | N/A | `https://ampcode.com/threads/{threadId}` |
| Permission system | `allowedTools` array | `createPermission()` helper |
| MCP servers | `createSdkMcpServer()` | `mcpConfig` object |

## Environment Variables

```bash
# Required: Amp API key
export AMP_API_KEY=sgamp_your_access_token_here

# Or use Amp CLI login
amp login
```

## Benefits

1. **Cost Reduction**: Rush mode is 67% cheaper per token
2. **Speed Improvement**: Rush mode is 50% faster
3. **Thread Sharing**: Debug sessions are shareable via URL
4. **Simplified Setup**: No need to locate Claude Code executable
5. **Better Model Selection**: Amp automatically picks the best model for the mode
6. **Thread Continuity**: Easy multi-turn debugging with thread handoff

## Decisions Made

| Question | Decision |
|----------|----------|
| Thread visibility | **Private by default**, public option for PRs |
| Mode switching | **Rush by default**, smart mode available for complex bugs |
| MCP integration | **mcpConfig object** instead of `createSdkMcpServer()` |
| Thread tagging | **Supported** with project, category, issueRef, and custom tags |

## Future Improvements

1. **Thread Cleanup**: Define retention policy for old debug threads
2. **Batch Debugging**: Run multiple debug agents in parallel for different scenarios
3. **Thread History**: Keep a history of debug threads per project
4. **Smart Mode Fallback**: Auto-switch to smart mode if rush fails to solve the issue
5. **Tag-based Search**: Query past debug threads by tags

## See Also

- [Amp SDK Documentation](https://ampcode.com/manual/sdk)
- [Rush Mode Announcement](https://ampcode.com/news/rush-mode)
- [Fleeter Daemon Design](../DAEMON_DESIGN.md)
