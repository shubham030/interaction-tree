/**
 * Agent executor using Claude Agent SDK.
 *
 * Uses the Claude Agent SDK which runs Claude Code as its runtime.
 * Authentication is handled by the Claude Code CLI in PATH.
 */
import type { AgentStreamEvent } from '../ws/protocol.js';
import type { SessionManager } from '../session/manager.js';
import type { SessionService } from '../session/service.js';
/** Context passed to the agent's MCP tools */
export interface AgentContext {
    sessionService?: SessionService;
    sessionManager?: SessionManager;
    getSessionService?: (sessionId: string) => SessionService | undefined;
    currentSessionId?: string;
    projectPath?: string;
}
export declare const DEFAULT_MODEL = "claude-haiku-4-5-20251001";
export interface AgentConfig {
    /** Model to use (defaults to claude-haiku-4-5-20251001) */
    model?: string;
    /** Max conversation turns before giving up */
    maxTurns?: number;
    /** Session idle timeout in ms */
    sessionTimeoutMs?: number;
}
export interface AgentExecutorConfig {
    /** Max turns before giving up */
    maxTurns: number;
    /** Working directory (session's projectPath) */
    cwd: string;
    /** Model to use (optional, defaults to SDK default) */
    model?: string;
    /** Resume an existing Claude SDK session */
    resume?: string;
}
export interface AgentExecutionResult {
    status: 'success' | 'error' | 'needs_context';
    summary?: string;
    error?: string;
    question?: string;
    suggestions?: string[];
    /** The Claude SDK session ID to use for resumption */
    sessionId?: string;
}
export type AgentStreamCallback = (event: Omit<AgentStreamEvent, 'type' | 'id' | 'sessionId'>) => void;
/**
 * Execute an agent with the interaction tree tools bound to a context.
 * Context includes VMClient, SessionManager, FlutterManager - all optional.
 */
export declare function executeAgent(systemPrompt: string, userMessage: string, config: AgentExecutorConfig, ctx: AgentContext, onEvent?: AgentStreamCallback): Promise<AgentExecutionResult>;
/**
 * Get the default agent config.
 */
export declare function getDefaultAgentConfig(cwd: string, overrides?: Partial<AgentConfig>): AgentExecutorConfig;
/**
 * AgentExecutor class that wraps agent execution for a session.
 * Maintains a single Claude SDK session per fleeter session for conversation continuity.
 */
export declare class AgentExecutor {
    private config;
    /** Claude SDK session ID for resuming conversations */
    private sdkSessionId?;
    constructor(config?: Partial<AgentConfig>);
    /** Get the current Claude SDK session ID */
    getSessionId(): string | undefined;
    /** Clear the session (start fresh conversation) */
    clearSession(): void;
    execute(options: {
        intent: string;
        sessionService?: SessionService;
        sessionManager?: SessionManager;
        cwd: string;
        onEvent?: AgentStreamCallback;
    }): Promise<AgentExecutionResult>;
}
/**
 * Tools explicitly blocked for the Debug Agent.
 * These are Amp's responsibility - the Debug Agent is runtime-only.
 */
export declare const DEBUG_AGENT_BLOCKED_TOOLS: string[];
/**
 * Execute the debug agent with RUNTIME-ONLY investigation tools.
 *
 * The debug agent investigates the RUNNING APP:
 * - Interact with the app, restart it, run it
 * - Get logs, errors, widget tree, widget states
 *
 * It CANNOT access source code or run shell commands.
 * It returns a debug report with observations and hypotheses.
 * Amp reads code and applies fixes based on the report.
 */
export declare function executeDebugAgent(userMessage: string, config: AgentExecutorConfig, ctx: AgentContext, onEvent?: AgentStreamCallback): Promise<AgentExecutionResult>;
/**
 * DebugAgentExecutor class that wraps debug agent execution.
 *
 * The debug agent is a RUNTIME-ONLY investigation agent that:
 * - Investigates running Flutter apps via Fleeter MCP tools
 * - Can run/restart the app, hot reload, execute interactions
 * - Gets logs, errors, widget tree, widget states
 * - Curates debug reports with runtime observations and hypotheses
 *
 * The debug agent CANNOT:
 * - Read source code (Read, Grep, glob, finder)
 * - Run shell commands (Bash)
 * - Edit source code (edit_file, create_file)
 *
 * These are Amp's responsibility after receiving the debug report.
 * Amp reads the report, searches code, and applies fixes.
 *
 * Maintains a Claude SDK session for multi-turn debugging conversations.
 */
export declare class DebugAgentExecutor {
    private config;
    /** Claude SDK session ID for resuming debug conversations */
    private sdkSessionId?;
    constructor(config?: Partial<AgentConfig>);
    /** Get the current Claude SDK session ID */
    getSessionId(): string | undefined;
    /** Clear the session (start fresh debug investigation) */
    clearSession(): void;
    /**
     * Execute a debug investigation.
     *
     * @param options.intent - What to debug (symptom, error, behavior)
     * @param options.sessionService - Fleeter session service for app interaction (optional, can be null if no session yet)
     * @param options.sessionManager - Session manager for session listing/creation
     * @param options.getSessionService - Function to dynamically get session service by ID
     * @param options.currentSessionId - Currently active session ID (if any)
     * @param options.projectPath - Flutter project path (for creating sessions)
     * @param options.cwd - Working directory (project path)
     * @param options.onEvent - Callback for streaming events
     */
    execute(options: {
        intent: string;
        sessionService?: SessionService;
        sessionManager?: SessionManager;
        getSessionService?: (sessionId: string) => SessionService | undefined;
        currentSessionId?: string;
        projectPath?: string;
        cwd: string;
        onEvent?: AgentStreamCallback;
    }): Promise<AgentExecutionResult>;
}
//# sourceMappingURL=executor.d.ts.map