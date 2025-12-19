/**
 * MCP Proxy - translates MCP protocol to daemon WebSocket commands.
 */
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema, } from '@modelcontextprotocol/sdk/types.js';
const TOOLS = [
    // Session management
    {
        name: 'create_session',
        description: 'Create a new Flutter session',
        inputSchema: {
            type: 'object',
            properties: {
                name: { type: 'string', description: 'Session name' },
                projectPath: { type: 'string', description: 'Path to Flutter project' },
            },
            required: ['name', 'projectPath'],
        },
    },
    {
        name: 'list_sessions',
        description: 'List all Flutter sessions',
        inputSchema: { type: 'object', properties: {} },
    },
    {
        name: 'connect_session',
        description: 'Connect to an existing session',
        inputSchema: {
            type: 'object',
            properties: {
                sessionId: { type: 'string', description: 'Session ID or name' },
            },
            required: ['sessionId'],
        },
    },
    {
        name: 'destroy_session',
        description: 'Destroy a session',
        inputSchema: {
            type: 'object',
            properties: {
                sessionId: { type: 'string', description: 'Session ID' },
            },
            required: ['sessionId'],
        },
    },
    // App lifecycle
    {
        name: 'run_app',
        description: 'Run Flutter app in current session',
        inputSchema: {
            type: 'object',
            properties: {
                device: { type: 'string', description: 'Target device' },
                flavor: { type: 'string', description: 'Build flavor' },
                target: { type: 'string', description: 'Target file' },
            },
        },
    },
    {
        name: 'stop_app',
        description: 'Stop Flutter app in current session',
        inputSchema: { type: 'object', properties: {} },
    },
    {
        name: 'hot_reload',
        description: 'Hot reload the running app',
        inputSchema: { type: 'object', properties: {} },
    },
    {
        name: 'hot_restart',
        description: 'Hot restart the running app',
        inputSchema: { type: 'object', properties: {} },
    },
    // Interaction tree
    {
        name: 'get_tree',
        description: 'Get interaction tree from running app',
        inputSchema: {
            type: 'object',
            properties: {
                summaryOnly: { type: 'boolean', description: 'Only include user widgets' },
            },
        },
    },
    {
        name: 'execute_interaction',
        description: 'Execute an interaction on a widget',
        inputSchema: {
            type: 'object',
            properties: {
                nodeId: { type: 'string', description: 'Target node ID' },
                interaction: { type: 'string', description: 'Interaction name' },
                args: { type: 'object', description: 'Interaction arguments' },
            },
            required: ['nodeId', 'interaction'],
        },
    },
    // Logs & Status
    {
        name: 'get_logs',
        description: 'Get Flutter app logs from current session',
        inputSchema: {
            type: 'object',
            properties: {
                maxLines: { type: 'number', description: 'Maximum number of log lines to return (default: 100)' },
            },
        },
    },
    {
        name: 'get_status',
        description: 'Get daemon and session status',
        inputSchema: { type: 'object', properties: {} },
    },
    // Widget state
    {
        name: 'get_state',
        description: 'Get the state of a specific widget by its node ID',
        inputSchema: {
            type: 'object',
            properties: {
                nodeId: { type: 'string', description: 'Target node ID' },
            },
            required: ['nodeId'],
        },
    },
    // Batch operations
    {
        name: 'batch',
        description: 'Execute multiple interactions in sequence',
        inputSchema: {
            type: 'object',
            properties: {
                steps: {
                    type: 'array',
                    description: 'Array of interaction steps to execute',
                    items: {
                        type: 'object',
                        properties: {
                            nodeId: { type: 'string', description: 'Target node ID' },
                            interaction: { type: 'string', description: 'Interaction name' },
                            args: { type: 'object', description: 'Interaction arguments' },
                            delayMs: { type: 'number', description: 'Delay in milliseconds before this step' },
                        },
                        required: ['nodeId', 'interaction'],
                    },
                },
            },
            required: ['steps'],
        },
    },
    // Error handling
    {
        name: 'get_errors',
        description: 'Get runtime errors from the Flutter app',
        inputSchema: { type: 'object', properties: {} },
    },
    // Session management (additional)
    {
        name: 'disconnect_session',
        description: 'Disconnect from the current session without destroying it',
        inputSchema: { type: 'object', properties: {} },
    },
    // Health
    {
        name: 'health_check',
        description: 'Check if the daemon is healthy and responding',
        inputSchema: { type: 'object', properties: {} },
    },
    // Context collection
    {
        name: 'get_context',
        description: 'Get complete app context for debugging/issue reproduction. Returns tree, recent logs, runtime errors, and app status in one call.',
        inputSchema: {
            type: 'object',
            properties: {
                maxLogs: { type: 'number', description: 'Maximum log lines to include (default: 50)' },
                summaryTree: { type: 'boolean', description: 'Only include user widgets in tree (default: true)' },
            },
        },
    },
    // Agent communication
    {
        name: 'send_agent_message',
        description: 'Send a message/intent to the Flutter agent. The agent will process it and the conversation will be visible in the TUI. Use this for natural language commands like "tap the login button" or "fill in the email field with test@example.com".',
        inputSchema: {
            type: 'object',
            properties: {
                message: { type: 'string', description: 'The message or intent to send to the agent' },
            },
            required: ['message'],
        },
    },
];
export class McpProxy {
    server;
    daemonClient;
    constructor(daemonClient) {
        this.daemonClient = daemonClient;
        this.server = new Server({ name: 'fleeter', version: '0.1.0' }, { capabilities: { tools: {} } });
        this.setupHandlers();
    }
    setupHandlers() {
        this.server.setRequestHandler(ListToolsRequestSchema, async () => {
            return { tools: TOOLS };
        });
        this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
            const { name, arguments: args } = request.params;
            try {
                // Special handling for send_agent_message -> maps to agent_message
                if (name === 'send_agent_message') {
                    const { message } = args;
                    const result = await this.daemonClient.sendCommand('agent_message', { intent: message });
                    return {
                        content: [{ type: 'text', text: JSON.stringify(result ?? { success: true }, null, 2) }],
                    };
                }
                const result = await this.daemonClient.sendCommand(name, args);
                return {
                    content: [{ type: 'text', text: JSON.stringify(result ?? { success: true }, null, 2) }],
                };
            }
            catch (err) {
                return {
                    content: [{ type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
                    isError: true,
                };
            }
        });
    }
    async start() {
        const transport = new StdioServerTransport();
        await this.server.connect(transport);
        console.error('[mcp-proxy] MCP server started');
    }
}
//# sourceMappingURL=proxy.js.map