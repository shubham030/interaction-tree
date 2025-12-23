/**
 * WebSocket protocol types for fleeter-daemon.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Client Types
// ─────────────────────────────────────────────────────────────────────────────

export type ClientType = 'mcp' | 'tui';

export interface ConnectedClient {
  id: string;
  type: ClientType;
  connectedAt: Date;
  currentSessionId: string | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Client → Daemon Messages
// ─────────────────────────────────────────────────────────────────────────────

export interface ClientHello {
  type: 'hello';
  clientType: ClientType;
  clientId: string;
  version: string;
}

export interface CommandMessage {
  type: 'command';
  id: string;
  clientId: string;
  action: CommandAction;
  data?: Record<string, unknown>;
}

export type CommandAction =
  // Session management
  | 'create_session'
  | 'destroy_session'
  | 'list_sessions'
  | 'connect_session'
  | 'disconnect_session'
  // App lifecycle
  | 'run_app'
  | 'stop_app'
  | 'hot_reload'
  | 'hot_restart'
  // Interaction tree
  | 'get_tree'
  | 'execute_interaction'
  | 'get_state'
  | 'batch'
  // Logs & errors
  | 'get_logs'
  | 'get_errors'
  // Context collection
  | 'get_context'
  // Agent
  | 'agent_message'
  | 'debug_agent_message'  // Debug agent (runtime-only investigation)
  // Status
  | 'get_status'
  | 'health_check';

export interface AgentToolCall {
  type: 'agent_tool_call';
  id: string;
  clientId: string;
  sessionId: string;
  intent: string;
  context?: string[];
  conversationId?: string;
}

export type ClientMessage = ClientHello | CommandMessage | AgentToolCall;

// ─────────────────────────────────────────────────────────────────────────────
// Daemon → Client Messages
// ─────────────────────────────────────────────────────────────────────────────

export interface ServerHello {
  type: 'hello_ack';
  daemonVersion: string;
  sessions: Array<{
    id: string;
    name: string;
    projectPath: string;
    appStatus: string;
  }>;
}

export interface CommandResponse {
  type: 'command_response';
  id: string;
  success: boolean;
  data?: unknown;
  error?: string;
}

export interface AgentResponse {
  type: 'agent_response';
  id: string;
  status: 'success' | 'failed' | 'needs_context';
  summary?: string;
  error?: string;
  question?: string;
  /** Claude SDK session ID for resumption (managed internally by daemon) */
  sdkSessionId?: string;
}

export interface MonitoringEvent {
  type: 'event';
  ts: string;
  source: 'flutter' | 'vm' | 'agent' | 'daemon' | 'session' | 'tree' | 'interaction';
  eventType: string;
  sessionId?: string;
  payload: unknown;
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Streaming Events
// ─────────────────────────────────────────────────────────────────────────────

export type AgentEventType =
  | { kind: 'text_delta'; text: string }
  | { kind: 'tool_call_start'; toolName: string; toolCallId: string }
  | { kind: 'tool_call_end'; toolName: string; toolCallId: string; result?: string }
  | { kind: 'message_complete' }  // Streaming finished for current message (immediate UI feedback)
  | { kind: 'task_complete'; summary: string }  // Full task/turn complete (after SDK finishes)
  | { kind: 'error'; message: string }
  | { kind: 'user_message'; text: string; clientId: string };  // User message from another client (e.g., MCP)

export interface AgentStreamEvent {
  type: 'agent_stream';
  id: string;
  sessionId: string;
  event: AgentEventType;
}

export type ServerMessage = ServerHello | CommandResponse | AgentResponse | MonitoringEvent | AgentStreamEvent;

// ─────────────────────────────────────────────────────────────────────────────
// Type Guards
// ─────────────────────────────────────────────────────────────────────────────

export function isClientHello(msg: unknown): msg is ClientHello {
  return typeof msg === 'object' && msg !== null && (msg as ClientHello).type === 'hello';
}

export function isCommandMessage(msg: unknown): msg is CommandMessage {
  return typeof msg === 'object' && msg !== null && (msg as CommandMessage).type === 'command';
}

export function isAgentToolCall(msg: unknown): msg is AgentToolCall {
  return typeof msg === 'object' && msg !== null && (msg as AgentToolCall).type === 'agent_tool_call';
}
