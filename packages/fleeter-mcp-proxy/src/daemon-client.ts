/**
 * WebSocket client for connecting to fleeter-daemon.
 */

import WebSocket from 'ws';
import { v4 as uuidv4 } from 'uuid';

export interface DaemonClientConfig {
  daemonUrl: string;
  clientId: string;
}

interface PendingRequest {
  resolve: (data: unknown) => void;
  reject: (error: Error) => void;
  timeout: NodeJS.Timeout;
}

export class DaemonClient {
  private ws: WebSocket | null = null;
  private config: DaemonClientConfig;
  private pendingRequests = new Map<string, PendingRequest>();
  private connected = false;
  private reconnecting = false;

  constructor(config: DaemonClientConfig) {
    this.config = config;
  }

  async connect(): Promise<void> {
    if (this.connected) return;

    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.config.daemonUrl);

      const timeout = setTimeout(() => {
        reject(new Error('Connection timeout'));
        this.ws?.close();
      }, 10000);

      this.ws.on('open', () => {
        clearTimeout(timeout);
        // Send hello
        this.ws!.send(JSON.stringify({
          type: 'hello',
          clientType: 'mcp',
          clientId: this.config.clientId,
          version: '0.1.0',
        }));
      });

      this.ws.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString());
          
          if (msg.type === 'hello_ack') {
            this.connected = true;
            console.error(`[mcp-proxy] Connected to daemon (${msg.sessions?.length ?? 0} sessions)`);
            resolve();
            return;
          }

          if (msg.type === 'command_response' && msg.id) {
            const pending = this.pendingRequests.get(msg.id);
            if (pending) {
              clearTimeout(pending.timeout);
              this.pendingRequests.delete(msg.id);
              if (msg.success) {
                pending.resolve(msg.data);
              } else {
                pending.reject(new Error(msg.error ?? 'Unknown error'));
              }
            }
          }

          if (msg.type === 'agent_response' && msg.id) {
            const pending = this.pendingRequests.get(msg.id);
            if (pending) {
              clearTimeout(pending.timeout);
              this.pendingRequests.delete(msg.id);
              pending.resolve(msg);
            }
          }
        } catch (err) {
          console.error(`[mcp-proxy] Error parsing message: ${err}`);
        }
      });

      this.ws.on('error', (err) => {
        clearTimeout(timeout);
        console.error(`[mcp-proxy] WebSocket error: ${err.message}`);
        if (!this.connected) {
          reject(err);
        }
      });

      this.ws.on('close', () => {
        this.connected = false;
        console.error('[mcp-proxy] Disconnected from daemon');
      });
    });
  }

  async sendCommand(action: string, data?: Record<string, unknown>): Promise<unknown> {
    if (!this.connected || !this.ws) {
      throw new Error('Not connected to daemon');
    }

    const id = uuidv4();
    // Timeout varies by action type:
    // - debug_agent_message: 10 min (agent runs Claude Code, multiple tool calls)
    // - run_app: 5 min (cold builds can be slow)
    // - default: 30 sec
    const timeoutMs = action === 'debug_agent_message' ? 600000 
                    : action === 'run_app' ? 300000 
                    : 30000;

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }, timeoutMs);

      this.pendingRequests.set(id, { resolve, reject, timeout });

      this.ws!.send(JSON.stringify({
        type: 'command',
        id,
        clientId: this.config.clientId,
        action,
        data,
      }));
    });
  }

  async sendAgentMessage(sessionId: string, intent: string, conversationId?: string): Promise<unknown> {
    if (!this.connected || !this.ws) {
      throw new Error('Not connected to daemon');
    }

    const id = uuidv4();

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error('Agent request timeout'));
      }, 120000); // 2 min for agent

      this.pendingRequests.set(id, { resolve, reject, timeout });

      this.ws!.send(JSON.stringify({
        type: 'agent_tool_call',
        id,
        clientId: this.config.clientId,
        sessionId,
        intent,
        conversationId,
      }));
    });
  }

  isConnected(): boolean {
    return this.connected;
  }

  close(): void {
    this.ws?.close();
    this.ws = null;
    this.connected = false;
  }
}
