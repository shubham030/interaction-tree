/**
 * Session types for fleeter-daemon.
 */
import type { AgentExecutor, DebugAgentExecutor } from '../agent/index.js';
export type AppStatus = 'not_running' | 'starting' | 'running' | 'stopped' | 'error';
export interface SessionInfo {
    id: string;
    name: string;
    projectPath: string;
    appStatus: AppStatus;
    vmServiceUri?: string;
    pid?: number;
    connectedClients: string[];
    createdAt: string;
    lastActiveAt: string;
}
export declare const MAX_SESSION_LOGS = 1000;
export declare const MAX_CHAT_HISTORY = 100;
/** Chat message role */
export type ChatRole = 'user' | 'assistant';
/** Chat message content types */
export type ChatContent = {
    type: 'text';
    text: string;
} | {
    type: 'tool_call';
    name: string;
    args: unknown;
    output?: string;
    status: 'running' | 'success' | 'failed';
};
/** A chat message in conversation history */
export interface ChatMessage {
    role: ChatRole;
    content: ChatContent;
    timestamp: string;
}
export interface Session {
    id: string;
    name: string;
    projectPath: string;
    appStatus: AppStatus;
    vmServiceUri?: string;
    pid?: number;
    connectedClients: Set<string>;
    createdAt: Date;
    lastActiveAt: Date;
    agent?: AgentExecutor;
    debugAgent?: DebugAgentExecutor;
    logs: string[];
    chatHistory: ChatMessage[];
}
export interface CreateSessionOptions {
    name: string;
    projectPath: string;
}
//# sourceMappingURL=types.d.ts.map