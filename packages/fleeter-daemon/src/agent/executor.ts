/**
 * Agent executor using Amp SDK.
 *
 * Uses the Amp SDK with rush mode for faster, cheaper debugging.
 * The debug agent runs on a separate Amp thread and returns the thread-id
 * for handoff to the main Amp instance.
 */

import {
  execute,
  createPermission,
  type Permission,
} from '@sourcegraph/amp-sdk';
import type { AgentStreamEvent } from '../ws/protocol.js';
import type { SessionManager } from '../session/manager.js';
import type { SessionService } from '../session/service.js';
import { generateToolbox, cleanupToolbox } from './toolbox-generator.js';

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

/**
 * Parse ASK_CONTEXT pattern from assistant response.
 */
function parseAskContext(content: string): {
  isAskContext: boolean;
  question?: string;
  suggestions?: string[];
} {
  const askMatch = content.match(/^\s*ASK_CONTEXT:\s*(.+?)(?:\n|$)/im);
  if (!askMatch) {
    return { isAskContext: false };
  }

  const question = askMatch[1].trim();
  const suggestionsMatch = content.match(/^\s*SUGGESTIONS:\s*(.+?)(?:\n|$)/im);
  const suggestions = suggestionsMatch
    ? suggestionsMatch[1].split(',').map((s) => s.trim())
    : undefined;

  return { isAskContext: true, question, suggestions };
}

/** Default daemon URI */
const DEFAULT_DAEMON_URI = 'ws://127.0.0.1:9877';

/**
 * Format thread tags for logging/display.
 */
function formatTags(tags?: ThreadTags): string {
  if (!tags) return '';
  const parts: string[] = [];
  if (tags.project) parts.push(`project:${tags.project}`);
  if (tags.category) parts.push(`category:${tags.category}`);
  if (tags.issueRef) parts.push(`issue:${tags.issueRef}`);
  if (tags.custom) parts.push(...tags.custom);
  return parts.join(', ');
}

export type AgentStreamCallback = (event: Omit<AgentStreamEvent, 'type' | 'id' | 'sessionId'>) => void;

/**
 * Debug Agent Allowed Tools - runtime investigation only via toolbox
 */
const DEBUG_AGENT_PERMISSIONS: Permission[] = [
  // Block source code access - Amp's responsibility
  createPermission('Read', 'reject'),
  createPermission('Grep', 'reject'),
  createPermission('glob', 'reject'),
  createPermission('finder', 'reject'),
  createPermission('Bash', 'reject'),
  createPermission('edit_file', 'reject'),
  createPermission('create_file', 'reject'),
];

/**
 * Tools explicitly blocked for the Debug Agent.
 */
export const DEBUG_AGENT_BLOCKED_TOOLS = [
  'Read',
  'Grep',
  'glob',
  'finder',
  'Bash',
  'edit_file',
  'create_file',
];

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
export async function executeDebugAgent(
  userMessage: string,
  config: AgentExecutorConfig,
  ctx: AgentContext,
  onEvent?: AgentStreamCallback
): Promise<AgentExecutionResult> {
  const { DEBUG_AGENT_SYSTEM_PROMPT } = await import('./prompts.js');

  let threadId: string | undefined;
  const textBlocks: string[] = [];
  const mode = config.mode ?? 'rush';
  const visibility = config.visibility ?? 'private';
  const tags = config.tags;

  // Generate toolbox scripts for this execution
  const daemonUri = config.daemonUri ?? DEFAULT_DAEMON_URI;
  const toolboxPath = generateToolbox(daemonUri);
  console.log(`[debug-agent] Generated toolbox at: ${toolboxPath}`);

  try {
    // Build the execute options with toolbox instead of MCP
    const executeOptions: Parameters<typeof execute>[0]['options'] = {
      cwd: config.cwd,
      dangerouslyAllowAll: true,
      permissions: DEBUG_AGENT_PERMISSIONS,
      toolbox: toolboxPath,
      logLevel: 'info',
    };

    // Continue existing thread if provided
    if (config.continueThread) {
      executeOptions.continue = config.continueThread;
    }

    // Build context with tags
    let contextInfo = '';
    if (tags) {
      const tagStr = formatTags(tags);
      if (tagStr) {
        contextInfo = `\n\n[Thread Tags: ${tagStr}]`;
      }
    }

    // Build the full prompt with system context
    const fullPrompt = `${DEBUG_AGENT_SYSTEM_PROMPT}${contextInfo}\n\n---\n\nUser Request: ${userMessage}`;

    console.log(`[debug-agent] Starting in ${mode} mode, visibility: ${visibility}, cwd: ${config.cwd}`);
    if (tags) {
      console.log(`[debug-agent] Tags: ${formatTags(tags)}`);
    }

    for await (const message of execute({ prompt: fullPrompt, options: executeOptions })) {
      // Capture thread ID from system init message
      if (message.type === 'system' && message.subtype === 'init') {
        const initMsg = message as { thread_id?: string; session_id?: string };
        threadId = initMsg.thread_id ?? initMsg.session_id;
        if (threadId) {
          console.log(`[debug-agent] Thread ID: ${threadId} (${visibility})`);
        }
      }

      // Handle assistant messages
      if (message.type === 'assistant') {
        const content = message.message?.content;
        if (Array.isArray(content)) {
          for (const block of content) {
            if (block.type === 'text') {
              textBlocks.push(block.text);
              onEvent?.({ event: { kind: 'text_delta', text: block.text } });
            }
            if (block.type === 'tool_use') {
              onEvent?.({
                event: {
                  kind: 'tool_call_start',
                  toolName: block.name ?? '',
                  toolCallId: block.id ?? '',
                },
              });
            }
          }
        }
      }

      // Handle tool results
      if (message.type === 'user') {
        const content = message.message?.content;
        if (Array.isArray(content)) {
          for (const block of content) {
            if (block.type === 'tool_result') {
              const resultText = Array.isArray(block.content)
                ? block.content.map((c: { type: string; text?: string }) => 
                    c.type === 'text' ? c.text : ''
                  ).join('')
                : typeof block.content === 'string' ? block.content : undefined;
              onEvent?.({
                event: {
                  kind: 'tool_call_end',
                  toolName: '',
                  toolCallId: block.tool_use_id,
                  result: resultText,
                },
              });
            }
          }
        }
      }

      // Handle final result
      if (message.type === 'result') {
        if (message.is_error) {
          const errorMsg = message.error ?? 'Unknown error';
          onEvent?.({ event: { kind: 'error', message: errorMsg } });
          return {
            status: 'error',
            error: errorMsg,
            threadId,
            threadUrl: threadId ? `https://ampcode.com/threads/${threadId}` : undefined,
            visibility,
            tags,
            sessionId: threadId,
          };
        }
      }
    }

    const allContent = textBlocks.join('\n');
    const finalContent = textBlocks.filter((t) => t.trim()).pop() ?? '';

    const askContext = parseAskContext(allContent);
    if (askContext.isAskContext) {
      return {
        status: 'needs_context',
        question: askContext.question,
        suggestions: askContext.suggestions,
        threadId,
        threadUrl: threadId ? `https://ampcode.com/threads/${threadId}` : undefined,
        visibility,
        tags,
        sessionId: threadId,
      };
    }

    onEvent?.({ event: { kind: 'task_complete', summary: finalContent } });

    return {
      status: 'success',
      summary: finalContent,
      threadId,
      threadUrl: threadId ? `https://ampcode.com/threads/${threadId}` : undefined,
      visibility,
      tags,
      sessionId: threadId,
    };
  } catch (err) {
    console.error('[debug-agent] executeDebugAgent error:', err);
    const errorMsg = err instanceof Error ? err.message : String(err);
    onEvent?.({ event: { kind: 'error', message: errorMsg } });
    return {
      status: 'error',
      error: errorMsg,
      threadId,
      threadUrl: threadId ? `https://ampcode.com/threads/${threadId}` : undefined,
      visibility,
      tags,
      sessionId: threadId,
    };
  } finally {
    // Clean up the generated toolbox
    cleanupToolbox(toolboxPath);
    console.log(`[debug-agent] Cleaned up toolbox: ${toolboxPath}`);
  }
}

/**
 * Get the default agent config.
 */
export function getDefaultAgentConfig(
  cwd: string,
  overrides?: Partial<AgentConfig>
): AgentExecutorConfig {
  return {
    maxTurns: overrides?.maxTurns ?? 10,
    cwd,
    mode: overrides?.mode ?? 'rush',
    visibility: overrides?.visibility ?? 'private',
    tags: overrides?.tags,
  };
}

/**
 * DebugAgentExecutor class that wraps debug agent execution.
 * 
 * The debug agent is a RUNTIME-ONLY investigation agent that:
 * - Uses Amp SDK with rush mode (default) or smart mode
 * - Creates PRIVATE Amp threads (can be made public for PRs)
 * - Supports thread tagging for categorization
 * - Returns thread-id for handoff to main Amp instance
 */
export class DebugAgentExecutor {
  private config: Partial<AgentConfig>;
  /** Amp thread ID for multi-turn conversations */
  private threadId?: string;
  /** Current thread visibility */
  private visibility: ThreadVisibility = 'private';
  /** Current thread tags */
  private tags?: ThreadTags;

  constructor(config?: Partial<AgentConfig>) {
    this.config = config ?? { mode: 'rush', visibility: 'private' };
    this.visibility = config?.visibility ?? 'private';
    this.tags = config?.tags;
  }

  /** Get the current Amp thread ID */
  getThreadId(): string | undefined {
    return this.threadId;
  }

  /** Get the thread URL for handoff to main Amp */
  getThreadUrl(): string | undefined {
    return this.threadId ? `https://ampcode.com/threads/${this.threadId}` : undefined;
  }

  /** Get the current thread visibility */
  getVisibility(): ThreadVisibility {
    return this.visibility;
  }

  /** Set thread visibility (for making public when creating PR) */
  setVisibility(visibility: ThreadVisibility): void {
    this.visibility = visibility;
  }

  /** Get current thread tags */
  getTags(): ThreadTags | undefined {
    return this.tags;
  }

  /** Set thread tags */
  setTags(tags: ThreadTags): void {
    this.tags = tags;
  }

  /** Add a tag to the current thread */
  addTag(key: keyof ThreadTags, value: string): void {
    if (!this.tags) {
      this.tags = {};
    }
    if (key === 'custom') {
      this.tags.custom = this.tags.custom ?? [];
      this.tags.custom.push(value);
    } else {
      this.tags[key] = value;
    }
  }

  /** Clear the thread (start fresh investigation) */
  clearThread(): void {
    this.threadId = undefined;
  }

  /** @deprecated Use clearThread instead - legacy compatibility */
  clearSession(): void {
    this.clearThread();
  }

  /** Switch agent mode (rush or smart) */
  setMode(mode: AgentMode): void {
    this.config.mode = mode;
  }

  /** Get current agent mode */
  getMode(): AgentMode {
    return this.config.mode ?? 'rush';
  }

  /**
   * Execute a debug investigation.
   * 
   * @returns Result including threadId, threadUrl, visibility, and tags
   */
  async execute(options: {
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
  }): Promise<AgentExecutionResult> {
    const { intent, cwd, onEvent } = options;

    // Merge tags
    const executionTags: ThreadTags = {
      ...this.tags,
      ...options.tags,
      custom: [...(this.tags?.custom ?? []), ...(options.tags?.custom ?? [])],
    };

    const config: AgentExecutorConfig = {
      ...getDefaultAgentConfig(cwd, this.config),
      mode: options.mode ?? this.config.mode ?? 'rush',
      visibility: options.visibility ?? this.visibility,
      tags: Object.keys(executionTags).length > 0 ? executionTags : undefined,
    };

    // Continue existing thread for multi-turn debugging
    if (this.threadId) {
      config.continueThread = this.threadId;
    }

    const ctx: AgentContext = {
      sessionService: options.sessionService,
      sessionManager: options.sessionManager,
      getSessionService: options.getSessionService,
      currentSessionId: options.currentSessionId,
      projectPath: options.projectPath,
    };

    const result = await executeDebugAgent(intent, config, ctx, onEvent);

    // Store thread ID and update visibility/tags
    if (result.threadId) {
      this.threadId = result.threadId;
    }
    if (result.visibility) {
      this.visibility = result.visibility;
    }
    if (result.tags) {
      this.tags = result.tags;
    }

    return result;
  }

  /**
   * Make the current thread public (e.g., for attaching to a PR).
   * Returns the public thread URL.
   */
  makePublic(): string | undefined {
    this.visibility = 'public';
    // Note: Actual visibility change would need Amp API call
    // For now, we just track the intent locally
    return this.getThreadUrl();
  }
}

// Legacy exports for backward compatibility
export { DebugAgentExecutor as AgentExecutor };
