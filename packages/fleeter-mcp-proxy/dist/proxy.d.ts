/**
 * MCP Proxy - translates MCP protocol to daemon WebSocket commands.
 *
 * Exposes a single tool: send_debug_message
 * The Debug Agent handles everything: sessions, app lifecycle, investigation.
 */
import { DaemonClient } from './daemon-client.js';
export declare class McpProxy {
    private server;
    private daemonClient;
    constructor(daemonClient: DaemonClient);
    private setupHandlers;
    start(): Promise<void>;
}
//# sourceMappingURL=proxy.d.ts.map