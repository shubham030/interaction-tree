/**
 * Agent executor using Amp SDK.
 *
 * Uses the Amp SDK with rush mode for faster, cheaper debugging.
 * The debug agent runs on a separate Amp thread and returns the thread-id
 * for handoff to the main Amp instance.
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
/** Thread visibility options */
export type ThreadVisibility = 'private' | 'public';
/** Agent mode - rush is faster/cheaper, smart is more capable */
export type AgentMode = 'rush' | 'smart';
/** Thread tags for categorization */
export interface ThreadTags {
    /** Project name (e.g., "tutero-app") */
    project?: string;
    /** Bug category (e.g., "cart", "auth", "ui") */
    category?: string;
    /** Issue/ticket reference (e.g., "TUTERO-123") */
    issueRef?: string;
    /** Custom tags */
    custom?: string[];
}
export interface AgentConfig {
    /** Agent mode - rush (default) or smart */
    mode?: AgentMode;
    /** Max conversation turns before giving up */
    maxTurns?: number;
    /** Session idle timeout in ms */
    sessionTimeoutMs?: number;
    /** Thread visibility - private (default) or public */
    visibility?: ThreadVisibility;
    /** Thread tags for categorization */
    tags?: ThreadTags;
}
export interface AgentExecutorConfig {
    /** Max turns before giving up */
    maxTurns: number;
    /** Working directory (session's projectPath) */
    cwd: string;
    /** Agent mode - rush (default) or smart */
    mode?: AgentMode;
    /** Continue an existing Amp thread by ID (format: T-xxx-xxx) */
    continueThread?: string;
    /** Thread visibility - private (default) or public */
    visibility?: ThreadVisibility;
    /** Thread tags for categorization */
    tags?: ThreadTags;
    /** Daemon WebSocket URI for toolbox scripts */
    daemonUri?: string;
}
export interface AgentExecutionResult {
    status: 'success' | 'error' | 'needs_context';
    summary?: string;
    error?: string;
    question?: string;
    suggestions?: string[];
    /** The Amp thread ID for handoff to main Amp instance (format: T-xxx-xxx) */
    threadId?: string;
    /** Full thread URL for easy reference */
    threadUrl?: string;
    /** Thread visibility */
    visibility?: ThreadVisibility;
    /** Thread tags */
    tags?: ThreadTags;
    /** @deprecated Use threadId instead - legacy compatibility */
    sessionId?: string;
}
export type AgentStreamCallback = (event: Omit<AgentStreamEvent, 'type' | 'id' | 'sessionId'>) => void;
/**
 * Tools explicitly blocked for the Debug Agent.
 */
export declare const DEBUG_AGENT_BLOCKED_TOOLS: string[];
/**
 * Execute the debug agent using Amp SDK.
 *
 * The debug agent:
 * - Runs in rush mode by default (faster, cheaper) - switchable to smart
 * - Creates a PRIVATE Amp thread (can be made public for PRs)
 * - Supports thread tagging for categorization
 * - Returns the thread-id for handoff to main Amp instance
 * - Uses toolbox scripts to access daemon tools (not MCP)
 */
export declare function executeDebugAgent(userMessage: string, config: AgentExecutorConfig, ctx: AgentContext, onEvent?: AgentStreamCallback): Promise<AgentExecutionResult>;
/**
 * Get the default agent config.
 */
export declare function getDefaultAgentConfig(cwd: string, overrides?: Partial<AgentConfig>): AgentExecutorConfig;
/**
 * DebugAgentExecutor class that wraps debug agent execution.
 *
 * The debug agent is a RUNTIME-ONLY investigation agent that:
 * - Uses Amp SDK with rush mode (default) or smart mode
 * - Creates PRIVATE Amp threads (can be made public for PRs)
 * - Supports thread tagging for categorization
 * - Returns thread-id for handoff to main Amp instance
 */
export declare class DebugAgentExecutor {
    private config;
    /** Amp thread ID for multi-turn conversations */
    private threadId?;
    /** Current thread visibility */
    private visibility;
    /** Current thread tags */
    private tags?;
    constructor(config?: Partial<AgentConfig>);
    /** Get the current Amp thread ID */
    getThreadId(): string | undefined;
    /** Get the thread URL for handoff to main Amp */
    getThreadUrl(): string | undefined;
    /** Get the current thread visibility */
    getVisibility(): ThreadVisibility;
    /** Set thread visibility (for making public when creating PR) */
    setVisibility(visibility: ThreadVisibility): void;
    /** Get current thread tags */
    getTags(): ThreadTags | undefined;
    /** Set thread tags */
    setTags(tags: ThreadTags): void;
    /** Add a tag to the current thread */
    addTag(key: keyof ThreadTags, value: string): void;
    /** Clear the thread (start fresh investigation) */
    clearThread(): void;
    /** @deprecated Use clearThread instead - legacy compatibility */
    clearSession(): void;
    /** Switch agent mode (rush or smart) */
    setMode(mode: AgentMode): void;
    /** Get current agent mode */
    getMode(): AgentMode;
    /**
     * Execute a debug investigation.
     *
     * @returns Result including threadId, threadUrl, visibility, and tags
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
        /** Override mode for this execution only */
        mode?: AgentMode;
        /** Override visibility for this execution only */
        visibility?: ThreadVisibility;
        /** Override/add tags for this execution */
        tags?: ThreadTags;
    }): Promise<AgentExecutionResult>;
    /**
     * Make the current thread public (e.g., for attaching to a PR).
     * Returns the public thread URL.
     */
    makePublic(): string | undefined;
}
export { DebugAgentExecutor as AgentExecutor };
//# sourceMappingURL=executor.d.ts.map