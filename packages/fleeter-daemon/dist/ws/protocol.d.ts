/**
 * WebSocket protocol types for fleeter-daemon.
 */
export type ClientType = 'mcp' | 'tui';
export interface ConnectedClient {
    id: string;
    type: ClientType;
    connectedAt: Date;
    currentSessionId: string | null;
}
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
export type CommandAction = 'create_session' | 'destroy_session' | 'list_sessions' | 'connect_session' | 'disconnect_session' | 'run_app' | 'stop_app' | 'hot_reload' | 'hot_restart' | 'get_tree' | 'execute_interaction' | 'get_state' | 'batch' | 'get_logs' | 'get_errors' | 'get_context' | 'agent_message' | 'debug_agent_message' | 'get_status' | 'health_check';
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
export type AgentEventType = {
    kind: 'text_delta';
    text: string;
} | {
    kind: 'tool_call_start';
    toolName: string;
    toolCallId: string;
} | {
    kind: 'tool_call_end';
    toolName: string;
    toolCallId: string;
    result?: string;
} | {
    kind: 'message_complete';
} | {
    kind: 'task_complete';
    summary: string;
} | {
    kind: 'error';
    message: string;
} | {
    kind: 'user_message';
    text: string;
    clientId: string;
};
export interface AgentStreamEvent {
    type: 'agent_stream';
    id: string;
    sessionId: string;
    event: AgentEventType;
}
export type ServerMessage = ServerHello | CommandResponse | AgentResponse | MonitoringEvent | AgentStreamEvent;
export declare function isClientHello(msg: unknown): msg is ClientHello;
export declare function isCommandMessage(msg: unknown): msg is CommandMessage;
export declare function isAgentToolCall(msg: unknown): msg is AgentToolCall;
//# sourceMappingURL=protocol.d.ts.map