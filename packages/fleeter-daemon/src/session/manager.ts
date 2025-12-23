/**
 * SessionManager - manages sessions shared across all daemon clients.
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import type { Session, SessionInfo, CreateSessionOptions, ChatMessage } from './types.js';
import { MAX_SESSION_LOGS, MAX_CHAT_HISTORY } from './types.js';
import { AgentExecutor, DebugAgentExecutor } from '../agent/index.js';

export class SessionManager extends EventEmitter {
  private sessions = new Map<string, Session>();

  /**
   * Create a new session.
   */
  create(options: CreateSessionOptions): SessionInfo {
    // Check for duplicate name
    for (const session of this.sessions.values()) {
      if (session.name === options.name) {
        throw new Error(`Session with name "${options.name}" already exists`);
      }
    }

    const id = uuidv4();
    const now = new Date();

    const session: Session = {
      id,
      name: options.name,
      projectPath: options.projectPath,
      appStatus: 'not_running',
      connectedClients: new Set(),
      createdAt: now,
      lastActiveAt: now,
      agent: new AgentExecutor(),  // Create agent for this session
      debugAgent: new DebugAgentExecutor(),  // Create debug agent (runtime-only investigation)
      logs: [],  // Session-level logs persist across reconnects
      chatHistory: [],  // Conversation history for TUI
    };

    this.sessions.set(id, session);
    console.error(`[session-manager] Created session "${options.name}" (${id})`);
    this.emit('session:created', this.toInfo(session));

    return this.toInfo(session);
  }

  /**
   * Destroy a session.
   */
  async destroy(nameOrId: string): Promise<void> {
    const session = this.get(nameOrId);
    if (!session) {
      throw new Error(`Session not found: ${nameOrId}`);
    }

    // Notify connected clients
    this.emit('session:destroying', session.id);

    this.sessions.delete(session.id);
    console.error(`[session-manager] Destroyed session "${session.name}"`);
    this.emit('session:destroyed', session.id);
  }

  /**
   * List all sessions.
   */
  list(): SessionInfo[] {
    return Array.from(this.sessions.values()).map((s) => this.toInfo(s));
  }

  /**
   * Get a session by name or ID.
   */
  get(nameOrId: string): Session | undefined {
    // Try by ID first
    if (this.sessions.has(nameOrId)) {
      return this.sessions.get(nameOrId);
    }
    // Try by name
    for (const session of this.sessions.values()) {
      if (session.name === nameOrId) {
        return session;
      }
    }
    return undefined;
  }

  /**
   * Connect a client to a session.
   */
  connectClient(sessionNameOrId: string, clientId: string): SessionInfo {
    const session = this.get(sessionNameOrId);
    if (!session) {
      throw new Error(`Session not found: ${sessionNameOrId}`);
    }

    session.connectedClients.add(clientId);
    session.lastActiveAt = new Date();
    console.error(`[session-manager] Client ${clientId} connected to session "${session.name}"`);
    this.emit('session:client_connected', session.id, clientId);

    return this.toInfo(session);
  }

  /**
   * Disconnect a client from a session.
   */
  disconnectClient(sessionNameOrId: string, clientId: string): void {
    const session = this.get(sessionNameOrId);
    if (!session) return;

    session.connectedClients.delete(clientId);
    console.error(`[session-manager] Client ${clientId} disconnected from session "${session.name}"`);
    this.emit('session:client_disconnected', session.id, clientId);
  }

  /**
   * Disconnect a client from all sessions.
   */
  disconnectClientFromAll(clientId: string): void {
    for (const session of this.sessions.values()) {
      if (session.connectedClients.has(clientId)) {
        this.disconnectClient(session.id, clientId);
      }
    }
  }

  /**
   * Get sessions a client is connected to.
   */
  getClientSessions(clientId: string): SessionInfo[] {
    return Array.from(this.sessions.values())
      .filter((s) => s.connectedClients.has(clientId))
      .map((s) => this.toInfo(s));
  }

  /**
   * Update session status.
   */
  updateStatus(sessionId: string, status: Session['appStatus'], details?: { vmServiceUri?: string; pid?: number; error?: string }): void {
    const session = this.get(sessionId);
    if (!session) {
      console.error(`[session-manager] updateStatus: session not found for ${sessionId}`);
      return;
    }
    console.error(`[session-manager] updateStatus: ${session.name} -> ${status}`);

    session.appStatus = status;
    if (details?.vmServiceUri) session.vmServiceUri = details.vmServiceUri;
    if (details?.pid) session.pid = details.pid;
    session.lastActiveAt = new Date();

    this.emit('session:status_changed', this.toInfo(session));
  }

  /**
   * Update session when VM client disconnects (but process may still be running).
   * This happens during hot restart or if VM service crashes.
   */
  updateVmDisconnected(sessionId: string): void {
    const session = this.get(sessionId);
    if (!session) {
      console.error(`[session-manager] updateVmDisconnected: session not found for ${sessionId}`);
      return;
    }
    console.error(`[session-manager] updateVmDisconnected: ${session.name}`);

    // Clear vmServiceUri but keep status as "running" if process is still alive
    // The Flutter process might be restarting and will reconnect
    session.vmServiceUri = undefined;
    session.lastActiveAt = new Date();

    this.emit('session:status_changed', this.toInfo(session));
  }

  /**
   * Add a log line to a session. Logs persist across process restarts.
   */
  addLog(sessionId: string, line: string): void {
    const session = this.get(sessionId);
    if (!session) return;

    session.logs.push(line);
    if (session.logs.length > MAX_SESSION_LOGS) {
      session.logs.shift();
    }
  }

  /**
   * Get logs for a session.
   */
  getLogs(sessionId: string, maxLines = 100): string[] {
    const session = this.sessions.get(sessionId);
    if (!session) return [];
    return session.logs.slice(-maxLines);
  }

  /**
   * Clear logs for a session.
   */
  clearLogs(sessionId: string): void {
    const session = this.sessions.get(sessionId);
    if (session) {
      session.logs = [];
    }
  }

  /**
   * Add a chat message to a session's history.
   * Deduplicates identical consecutive messages from the same role.
   */
  addChatMessage(sessionId: string, message: ChatMessage): void {
    const session = this.get(sessionId);
    if (!session) return;

    // Deduplicate: skip if last message has same role and content
    const last = session.chatHistory[session.chatHistory.length - 1];
    if (last && last.role === message.role) {
      const lastContent = JSON.stringify(last.content);
      const newContent = JSON.stringify(message.content);
      if (lastContent === newContent) {
        return; // Skip duplicate
      }
    }

    session.chatHistory.push(message);
    if (session.chatHistory.length > MAX_CHAT_HISTORY) {
      session.chatHistory.shift();
    }
  }

  /**
   * Get chat history for a session.
   */
  getChatHistory(sessionId: string): ChatMessage[] {
    const session = this.sessions.get(sessionId);
    if (!session) return [];
    return session.chatHistory;
  }

  /**
   * Clear chat history for a session.
   */
  clearChatHistory(sessionId: string): void {
    const session = this.sessions.get(sessionId);
    if (session) {
      session.chatHistory = [];
      // Also clear the agent sessions to start fresh conversation
      session.agent?.clearSession();
      session.debugAgent?.clearSession();
    }
  }

  toInfo(session: Session): SessionInfo {
    return {
      id: session.id,
      name: session.name,
      projectPath: session.projectPath,
      appStatus: session.appStatus,
      vmServiceUri: session.vmServiceUri,
      pid: session.pid,
      connectedClients: Array.from(session.connectedClients),
      createdAt: session.createdAt.toISOString(),
      lastActiveAt: session.lastActiveAt.toISOString(),
    };
  }
}
