/**
 * MCP Proxy - translates MCP protocol to daemon WebSocket commands.
 * 
 * Exposes a single tool: send_debug_message
 * The Debug Agent handles everything: sessions, app lifecycle, investigation.
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { DaemonClient } from './daemon-client.js';

const TOOLS = [
  {
    name: 'send_debug_message',
    description: `Interact with a Flutter app via the Debug Agent.

The Debug Agent is a FULLY AUTONOMOUS Flutter runtime expert that handles:
- Session management (creates session if needed, connects to existing)
- App lifecycle (runs app if not running, hot reload/restart)
- Runtime investigation (logs, errors, widget tree, widget states)
- Fix validation (applies changes, tests, reports results)

Use this for ALL Flutter-related tasks:
- "Debug why the cart doesn't update when removing items"
- "Validate my fix (I added notifyListeners to removeItem)"
- "Test the checkout flow and report issues"
- "Tap the login button and check for errors"
- "What's the current screen state?"

The Debug Agent returns structured reports with:
- Runtime observations (logs, errors, widget states)
- Reproduction steps with results
- Hypothesis based on runtime behavior
- Keywords to search in source code (for investigation)
- Validation results (for fix validation)

After receiving an investigation report, YOU (Amp) should:
1. Read source code based on keywords provided
2. Apply the fix
3. Ask Debug Agent to validate the fix

The session and app persist between calls - the Debug Agent reconnects automatically.

IMPORTANT: Always provide the projectPath when debugging a Flutter app.`,
    inputSchema: {
      type: 'object' as const,
      properties: {
        message: { 
          type: 'string', 
          description: 'What to do in the Flutter app (debug, validate, interact, test)' 
        },
        projectPath: {
          type: 'string',
          description: 'Absolute path to the Flutter project directory. Required for creating sessions.',
        },
      },
      required: ['message', 'projectPath'],
    },
  },
];

export class McpProxy {
  private server: Server;
  private daemonClient: DaemonClient;

  constructor(daemonClient: DaemonClient) {
    this.daemonClient = daemonClient;
    this.server = new Server(
      { name: 'fleeter', version: '0.1.0' },
      { capabilities: { tools: {} } }
    );

    this.setupHandlers();
  }

  private setupHandlers(): void {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return { tools: TOOLS };
    });

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        if (name === 'send_debug_message') {
          const { message, projectPath } = args as { message: string; projectPath: string };
          const result = await this.daemonClient.sendCommand('debug_agent_message', { 
            intent: message,
            projectPath,
          });
          return {
            content: [{ type: 'text', text: JSON.stringify(result ?? { success: true }, null, 2) }],
          };
        }

        return {
          content: [{ type: 'text', text: `Unknown tool: ${name}` }],
          isError: true,
        };
      } catch (err) {
        return {
          content: [{ type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
          isError: true,
        };
      }
    });
  }

  async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('[mcp-proxy] MCP server started');
  }
}
