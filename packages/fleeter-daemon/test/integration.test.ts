/**
 * Integration tests for fleeter-daemon.
 * 
 * These tests verify the full flow from WebSocket connection through to
 * session and app management.
 */

import WebSocket from 'ws';
import { v4 as uuidv4 } from 'uuid';
import { describe, test, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import path from 'path';

const DAEMON_PORT = process.env.DAEMON_PORT || 9877;
const DAEMON_URL = `ws://127.0.0.1:${DAEMON_PORT}`;
const EXAMPLE_PROJECT = path.resolve(__dirname, '../../../example');

interface ServerHello {
  type: 'hello_ack';
  daemonVersion: string;
  sessions: Array<{
    id: string;
    name: string;
    projectPath: string;
    appStatus: string;
  }>;
}

interface CommandResponse {
  type: 'command_response';
  id: string;
  success: boolean;
  data?: unknown;
  error?: string;
}

class TestClient {
  private ws: WebSocket | null = null;
  private clientId = uuidv4();
  private messageQueue: Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }> = new Map();
  private events: unknown[] = [];

  async connect(): Promise<ServerHello> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(DAEMON_URL);
      
      this.ws.on('error', (err) => reject(err));
      
      this.ws.on('open', () => {
        this.ws!.send(JSON.stringify({
          type: 'hello',
          clientType: 'test',
          clientId: this.clientId,
          version: '0.1.0',
        }));
      });

      this.ws.on('message', (data) => {
        const msg = JSON.parse(data.toString());
        
        if (msg.type === 'hello_ack') {
          resolve(msg as ServerHello);
        } else if (msg.type === 'command_response') {
          const pending = this.messageQueue.get(msg.id);
          if (pending) {
            this.messageQueue.delete(msg.id);
            pending.resolve(msg);
          }
        } else if (msg.type === 'event') {
          this.events.push(msg);
        }
      });
    });
  }

  async sendCommand(action: string, data?: unknown): Promise<CommandResponse> {
    const id = uuidv4();
    
    return new Promise((resolve, reject) => {
      this.messageQueue.set(id, { resolve: resolve as (v: unknown) => void, reject });
      
      this.ws!.send(JSON.stringify({
        type: 'command',
        id,
        clientId: this.clientId,
        action,
        data,
      }));

      // Timeout
      setTimeout(() => {
        if (this.messageQueue.has(id)) {
          this.messageQueue.delete(id);
          reject(new Error(`Command ${action} timed out`));
        }
      }, 30000);
    });
  }

  getEvents(): unknown[] {
    return this.events;
  }

  clearEvents(): void {
    this.events = [];
  }

  close(): void {
    this.ws?.close();
    this.ws = null;
  }
}

describe('Fleeter Daemon Integration Tests', () => {
  let client: TestClient;
  let createdSessionId: string | null = null;

  beforeAll(async () => {
    client = new TestClient();
  });

  afterAll(async () => {
    // Clean up created session
    if (createdSessionId) {
      try {
        await client.sendCommand('destroy_session', { sessionId: createdSessionId });
      } catch {
        // Ignore cleanup errors
      }
    }
    client.close();
  });

  test('should connect and receive hello_ack', async () => {
    const hello = await client.connect();
    
    expect(hello.type).toBe('hello_ack');
    expect(hello.daemonVersion).toBeDefined();
    expect(Array.isArray(hello.sessions)).toBe(true);
  });

  test('should list sessions', async () => {
    const response = await client.sendCommand('list_sessions');
    
    expect(response.success).toBe(true);
    expect(response.data).toHaveProperty('sessions');
    expect(Array.isArray((response.data as { sessions: unknown[] }).sessions)).toBe(true);
  });

  test('should create a session', async () => {
    const sessionName = `test-session-${Date.now()}`;
    const response = await client.sendCommand('create_session', {
      name: sessionName,
      projectPath: EXAMPLE_PROJECT,
    });
    
    expect(response.success).toBe(true);
    expect(response.data).toHaveProperty('session');
    
    const session = (response.data as { session: { id: string; name: string; projectPath: string; appStatus: string } }).session;
    expect(session.name).toBe(sessionName);
    expect(session.projectPath).toBe(EXAMPLE_PROJECT);
    expect(session.appStatus).toBe('not_running');
    
    createdSessionId = session.id;
  });

  test('should connect to a session', async () => {
    expect(createdSessionId).not.toBeNull();
    
    const response = await client.sendCommand('connect_session', {
      sessionId: createdSessionId,
    });
    
    expect(response.success).toBe(true);
    expect(response.data).toHaveProperty('session');
  });

  test('should get status', async () => {
    const response = await client.sendCommand('get_status');
    
    expect(response.success).toBe(true);
    expect(response.data).toHaveProperty('daemon');
    expect(response.data).toHaveProperty('currentSession');
    
    const data = response.data as { daemon: { version: string }; currentSession: unknown };
    expect(data.daemon.version).toBeDefined();
  });

  // NOTE: run_app test removed - it opens windows and doesn't verify app behavior meaningfully

  test('should destroy session', async () => {
    expect(createdSessionId).not.toBeNull();
    
    const response = await client.sendCommand('destroy_session', {
      sessionId: createdSessionId,
    });
    
    expect(response.success).toBe(true);
    createdSessionId = null;
  });

  test('should return error for unknown command', async () => {
    const response = await client.sendCommand('unknown_command');
    
    expect(response.success).toBe(false);
    expect(response.error).toContain('Unknown action');
  });

  test('should return error for missing required fields', async () => {
    const response = await client.sendCommand('create_session', {
      // Missing name and projectPath
    });
    
    expect(response.success).toBe(false);
    expect(response.error).toBeDefined();
  });
});
