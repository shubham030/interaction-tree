/**
 * Agent executor using Claude Agent SDK.
 *
 * Uses the Claude Agent SDK which runs Claude Code as its runtime.
 * Authentication is handled by the Claude Code CLI in PATH.
 */
import { query, createSdkMcpServer, tool, } from '@anthropic-ai/claude-agent-sdk';
import { z } from 'zod';
import { existsSync, readdirSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
export const DEFAULT_MODEL = 'claude-haiku-4-5-20251001';
/**
 * Parse ASK_CONTEXT pattern from assistant response.
 * More robust parsing: case-insensitive, handles multiline.
 */
function parseAskContext(content) {
    // Case-insensitive, anchored to line start (with optional whitespace)
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
/**
 * Create the interaction tree MCP server for the agent.
 * Takes context with SessionService (per-session) and SessionManager (global operations).
 *
 * The context includes a `getSessionService` function for dynamic lookup when
 * the agent creates or connects to sessions during execution.
 */
function createInteractionTreeMcpServer(ctx) {
    const { sessionManager, getSessionService, projectPath: ctxProjectPath } = ctx;
    // Track the current session ID - can be updated by connectSession/createSession
    let currentSessionId = ctx.currentSessionId;
    // Project path from context (for auto-creating sessions)
    const defaultProjectPath = ctxProjectPath;
    // Get the current session service - either from context or dynamic lookup
    const getCurrentSessionService = () => {
        if (currentSessionId && getSessionService) {
            return getSessionService(currentSessionId);
        }
        return ctx.sessionService;
    };
    const noSessionServiceError = {
        content: [{ type: 'text', text: 'Error: No session connected. Use listSessions to find sessions or createSession to create one, then use connectSession to connect.' }],
    };
    const noSessionManagerError = {
        content: [{ type: 'text', text: 'Error: No session manager available.' }],
    };
    const wrapError = (err, context) => ({
        content: [{ type: 'text', text: `Error ${context}: ${err instanceof Error ? err.message : String(err)}` }],
    });
    const getStatusTool = tool('getStatus', 'Get the current connection status. Returns vmConnected: true if the app is running and connected.', {}, async () => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return { content: [{ type: 'text', text: JSON.stringify({ connected: false, vmConnected: false, currentSessionId, message: 'No session connected' }, null, 2) }] };
        }
        try {
            const status = sessionService.getStatus();
            return {
                content: [{ type: 'text', text: JSON.stringify({ ...status, currentSessionId }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'getting status');
        }
    });
    const getTreeTool = tool('getTree', 'Get the interaction tree showing all interactable widgets in the app.', {
        includeBounds: z.boolean().optional(),
        includeState: z.boolean().optional(),
    }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const tree = await sessionService.getTree({
                includeBounds: args.includeBounds,
                includeWidgetType: true,
                includeState: args.includeState,
            });
            return {
                content: [{ type: 'text', text: JSON.stringify(tree, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'getting tree');
        }
    });
    const executeTool = tool('execute', 'Execute an interaction on a widget (tap, doubleTap, longPress, enterText, etc.)', {
        id: z.string(),
        interaction: z.string(),
        args: z.record(z.unknown()).optional(),
    }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const result = await sessionService.execute(args.id, args.interaction, args.args);
            return {
                content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, `executing ${args.interaction} on ${args.id}`);
        }
    });
    const getStateTool = tool('getState', 'Get the current state of a widget.', { id: z.string() }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const state = await sessionService.getState(args.id);
            return {
                content: [{ type: 'text', text: JSON.stringify(state, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, `getting state for ${args.id}`);
        }
    });
    const batchTool = tool('batch', 'Execute multiple interactions in sequence.', {
        steps: z.array(z.object({
            action: z.string(),
            id: z.string(),
            text: z.string().optional(),
            dx: z.number().optional(),
            dy: z.number().optional(),
            alignment: z.number().optional(),
            actionName: z.string().optional(),
            args: z.record(z.unknown()).optional(),
            condition: z.string().optional(),
            timeoutMs: z.number().optional(),
        })),
    }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const result = await sessionService.batch(args.steps);
            return {
                content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'executing batch');
        }
    });
    const hotReloadTool = tool('hotReload', 'Hot reload the app to apply code changes. Use for UI-only changes.', {}, async () => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const result = await sessionService.hotReload();
            return {
                content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'during hot reload');
        }
    });
    const hotRestartTool = tool('hotRestart', 'Hot restart the app (full restart, loses state). Use for state/logic changes.', {}, async () => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const result = await sessionService.hotRestart();
            return {
                content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'during hot restart');
        }
    });
    const getLogsTool = tool('getLogs', 'Get recent logs from the Flutter app.', { maxLines: z.number().optional() }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const logs = sessionService.getLogs(args.maxLines);
            return {
                content: [{ type: 'text', text: JSON.stringify({ logs }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'getting logs');
        }
    });
    const getErrorsTool = tool('getErrors', 'Get runtime errors from the app.', {}, async () => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            const errors = await sessionService.getRuntimeErrors();
            return {
                content: [{ type: 'text', text: JSON.stringify({ errors }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'getting errors');
        }
    });
    // Session management tools (use sessionManager for global operations)
    const createSessionTool = tool('createSession', 'Create a new Flutter session and automatically connect to it. If projectPath is not provided, uses the default project path.', {
        name: z.string().describe('Session name'),
        projectPath: z.string().optional().describe('Path to Flutter project (optional if default is set)'),
    }, async (args) => {
        if (!sessionManager) {
            return noSessionManagerError;
        }
        const finalProjectPath = args.projectPath ?? defaultProjectPath;
        if (!finalProjectPath) {
            return {
                content: [{ type: 'text', text: 'Error: No project path provided and no default project path set. Please provide a projectPath.' }],
            };
        }
        try {
            const session = sessionManager.create({ name: args.name, projectPath: finalProjectPath });
            // Auto-connect to the newly created session
            currentSessionId = session.id;
            return {
                content: [{ type: 'text', text: JSON.stringify({ created: true, connected: true, session }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'creating session');
        }
    });
    const listSessionsTool = tool('listSessions', 'List all Flutter sessions. Returns session IDs that can be used with connectSession.', {}, async () => {
        if (!sessionManager) {
            return noSessionManagerError;
        }
        const sessions = sessionManager.list();
        return {
            content: [{ type: 'text', text: JSON.stringify({ sessions, currentSessionId }, null, 2) }],
        };
    });
    const connectSessionTool = tool('connectSession', 'Connect to an existing session by name or ID. Required before using app interaction tools.', {
        sessionId: z.string().describe('Session ID or name'),
    }, async (args) => {
        if (!sessionManager) {
            return noSessionManagerError;
        }
        const session = sessionManager.get(args.sessionId);
        if (!session) {
            return {
                content: [{ type: 'text', text: `Error: Session not found: ${args.sessionId}` }],
            };
        }
        // Set the current session
        currentSessionId = session.id;
        return {
            content: [{ type: 'text', text: JSON.stringify({ connected: true, currentSessionId: session.id, session: sessionManager.toInfo(session) }, null, 2) }],
        };
    });
    const destroySessionTool = tool('destroySession', 'Destroy a session by name or ID.', {
        sessionId: z.string().describe('Session ID or name'),
    }, async (args) => {
        if (!sessionManager) {
            return noSessionManagerError;
        }
        try {
            await sessionManager.destroy(args.sessionId);
            // Clear current session if it was the one destroyed
            if (currentSessionId === args.sessionId) {
                currentSessionId = undefined;
            }
            return {
                content: [{ type: 'text', text: JSON.stringify({ destroyed: true }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'destroying session');
        }
    });
    // App lifecycle tools (use sessionService)
    const runAppTool = tool('runApp', 'Run the Flutter app in the current session. The app will start and connect to the VM service.', {
        device: z.string().optional().describe('Target device'),
        flavor: z.string().optional().describe('Build flavor'),
        target: z.string().optional().describe('Target file (e.g., lib/main.dart)'),
    }, async (args) => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            await sessionService.runApp({
                device: args.device,
                flavor: args.flavor,
                target: args.target,
            });
            return {
                content: [{ type: 'text', text: JSON.stringify({ success: true, message: 'App starting. Use getStatus to check when vmConnected is true.' }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'running app');
        }
    });
    const stopAppTool = tool('stopApp', 'Stop the Flutter app in the current session.', {}, async () => {
        const sessionService = getCurrentSessionService();
        if (!sessionService) {
            return noSessionServiceError;
        }
        try {
            await sessionService.stopApp();
            return {
                content: [{ type: 'text', text: JSON.stringify({ success: true }, null, 2) }],
            };
        }
        catch (err) {
            return wrapError(err, 'stopping app');
        }
    });
    return createSdkMcpServer({
        name: 'interaction-tree',
        version: '0.1.0',
        tools: [
            // Status & tree
            getStatusTool,
            getTreeTool,
            // Interactions
            executeTool,
            getStateTool,
            batchTool,
            // Hot reload/restart
            hotReloadTool,
            hotRestartTool,
            // Logs & errors
            getLogsTool,
            getErrorsTool,
            // Session management
            createSessionTool,
            listSessionsTool,
            connectSessionTool,
            destroySessionTool,
            // App lifecycle
            runAppTool,
            stopAppTool,
        ],
    });
}
/**
 * Execute an agent with the interaction tree tools bound to a context.
 * Context includes VMClient, SessionManager, FlutterManager - all optional.
 */
export async function executeAgent(systemPrompt, userMessage, config, ctx, onEvent) {
    const mcpServer = createInteractionTreeMcpServer(ctx);
    const options = {
        cwd: config.cwd,
        maxTurns: config.maxTurns,
        systemPrompt,
        mcpServers: {
            'interaction-tree': mcpServer,
        },
        allowedTools: [
            // Status & tree
            'mcp__interaction-tree__getStatus',
            'mcp__interaction-tree__getTree',
            // Interactions
            'mcp__interaction-tree__execute',
            'mcp__interaction-tree__getState',
            'mcp__interaction-tree__batch',
            // Hot reload/restart
            'mcp__interaction-tree__hotReload',
            'mcp__interaction-tree__hotRestart',
            // Logs & errors
            'mcp__interaction-tree__getLogs',
            'mcp__interaction-tree__getErrors',
            // Session management
            'mcp__interaction-tree__createSession',
            'mcp__interaction-tree__listSessions',
            'mcp__interaction-tree__connectSession',
            'mcp__interaction-tree__destroySession',
            // App lifecycle
            'mcp__interaction-tree__runApp',
            'mcp__interaction-tree__stopApp',
        ],
        permissionMode: 'bypassPermissions',
        allowDangerouslySkipPermissions: true,
        includePartialMessages: true, // Enable streaming
    };
    options.model = config.model ?? DEFAULT_MODEL;
    console.log(`[agent] Using model: ${options.model}`);
    if (config.resume) {
        options.resume = config.resume;
    }
    try {
        const textBlocks = [];
        let sessionId;
        for await (const message of query({ prompt: userMessage, options })) {
            // Capture session ID from the init message
            if (message.type === 'system' && message.subtype === 'init') {
                sessionId = message.session_id;
            }
            // Handle streaming partial messages (text deltas, block boundaries)
            if (message.type === 'stream_event') {
                const evt = message.event;
                // Text streaming
                if (evt.type === 'content_block_delta' && evt.delta?.type === 'text_delta') {
                    onEvent?.({ event: { kind: 'text_delta', text: evt.delta.text ?? '' } });
                }
                // Tool block starting - emit tool_call_start immediately during streaming
                // This preserves the correct ordering: text -> tool -> text
                if (evt.type === 'content_block_start' && evt.content_block?.type === 'tool_use') {
                    onEvent?.({ event: {
                            kind: 'tool_call_start',
                            toolName: evt.content_block.name ?? '',
                            toolCallId: evt.content_block.id ?? ''
                        } });
                }
                // Message streaming complete - emit message_complete for immediate UI feedback
                // Don't wait for the full loop to finish (which includes SDK overhead)
                if (evt.type === 'message_stop') {
                    onEvent?.({ event: { kind: 'message_complete' } });
                }
            }
            if (message.type === 'assistant') {
                for (const block of message.message.content) {
                    if (block.type === 'text') {
                        textBlocks.push(block.text);
                        // Don't emit text_delta here since we already streamed it via stream_event
                    }
                    // Note: tool_call_start is now emitted via stream_event content_block_start
                    // to ensure correct ordering during streaming
                }
            }
            if (message.type === 'user') {
                for (const block of message.message.content) {
                    if (block.type === 'tool_result') {
                        const resultText = Array.isArray(block.content)
                            ? block.content.map((c) => c.type === 'text' ? c.text : '').join('')
                            : typeof block.content === 'string' ? block.content : undefined;
                        onEvent?.({ event: { kind: 'tool_call_end', toolName: '', toolCallId: block.tool_use_id, result: resultText } });
                    }
                }
            }
            if (message.type === 'result') {
                if (message.subtype !== 'success') {
                    const errorMsg = 'errors' in message ? message.errors.join(', ') : 'Unknown error';
                    onEvent?.({ event: { kind: 'error', message: errorMsg } });
                    return {
                        status: 'error',
                        error: errorMsg,
                        sessionId,
                    };
                }
            }
        }
        const allContent = textBlocks.join('\n');
        const finalContent = textBlocks.filter(t => t.trim()).pop() ?? '';
        const askContext = parseAskContext(allContent);
        if (askContext.isAskContext) {
            return {
                status: 'needs_context',
                question: askContext.question,
                suggestions: askContext.suggestions,
                sessionId,
            };
        }
        onEvent?.({ event: { kind: 'task_complete', summary: finalContent } });
        return {
            status: 'success',
            summary: finalContent,
            sessionId,
        };
    }
    catch (err) {
        console.error('[agent] executeAgent error:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        onEvent?.({ event: { kind: 'error', message: errorMsg } });
        return {
            status: 'error',
            error: errorMsg,
        };
    }
}
/**
 * Get the default agent config.
 */
export function getDefaultAgentConfig(cwd, overrides) {
    return {
        maxTurns: overrides?.maxTurns ?? 10,
        cwd,
        model: overrides?.model,
    };
}
/**
 * AgentExecutor class that wraps agent execution for a session.
 * Maintains a single Claude SDK session per fleeter session for conversation continuity.
 */
export class AgentExecutor {
    config;
    /** Claude SDK session ID for resuming conversations */
    sdkSessionId;
    constructor(config) {
        this.config = config ?? {};
    }
    /** Get the current Claude SDK session ID */
    getSessionId() {
        return this.sdkSessionId;
    }
    /** Clear the session (start fresh conversation) */
    clearSession() {
        this.sdkSessionId = undefined;
    }
    async execute(options) {
        const { intent, sessionService, sessionManager, cwd, onEvent } = options;
        const { AGENT_SYSTEM_PROMPT } = await import('./prompts.js');
        const config = getDefaultAgentConfig(cwd, this.config);
        // Resume existing session if we have one
        if (this.sdkSessionId) {
            config.resume = this.sdkSessionId;
        }
        const ctx = {
            sessionService,
            sessionManager,
        };
        const result = await executeAgent(AGENT_SYSTEM_PROMPT, intent, config, ctx, onEvent);
        // Store the session ID for future resumption
        if (result.sessionId) {
            this.sdkSessionId = result.sessionId;
        }
        return result;
    }
}
/**
 * Debug Agent Allowed Tools
 *
 * The Debug Agent is a RUNTIME-ONLY investigator. It can ONLY use:
 * - Fleeter MCP tools (interact with app, reload, restart, run, stop, sessions)
 *
 * It CANNOT access (Amp's responsibility):
 * - File reading: Read, Grep, glob, finder (Amp reads code based on report)
 * - Bash: flutter analyze, git blame, etc. (Amp runs these)
 * - edit_file, create_file (Amp applies code fixes)
 */
const DEBUG_AGENT_ALLOWED_TOOLS = [
    // App investigation & control - ALL Fleeter MCP tools
    'mcp__interaction-tree__getStatus',
    'mcp__interaction-tree__getTree',
    'mcp__interaction-tree__execute',
    'mcp__interaction-tree__getState',
    'mcp__interaction-tree__batch',
    'mcp__interaction-tree__getLogs',
    'mcp__interaction-tree__getErrors',
    // App lifecycle - needed to reproduce issues
    'mcp__interaction-tree__hotReload',
    'mcp__interaction-tree__hotRestart',
    'mcp__interaction-tree__runApp',
    'mcp__interaction-tree__stopApp',
    // Session management - might need to create session to debug
    'mcp__interaction-tree__createSession',
    'mcp__interaction-tree__listSessions',
    'mcp__interaction-tree__connectSession',
    'mcp__interaction-tree__destroySession',
    // NO source code access - Amp reads code based on the debug report
    // NO shell commands - Amp runs flutter analyze, tests, etc.
];
/**
 * Tools explicitly blocked for the Debug Agent.
 * These are Amp's responsibility - the Debug Agent is runtime-only.
 */
export const DEBUG_AGENT_BLOCKED_TOOLS = [
    // Code access - Amp reads code based on debug report
    'Read',
    'Grep',
    'glob',
    'finder',
    // Shell commands - Amp runs flutter analyze, tests, etc.
    'Bash',
    // Code modification - Amp applies fixes
    'edit_file',
    'create_file',
];
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
export async function executeDebugAgent(userMessage, config, ctx, onEvent) {
    const { DEBUG_AGENT_SYSTEM_PROMPT } = await import('./prompts.js');
    const mcpServer = createInteractionTreeMcpServer(ctx);
    // Find Claude Code executable - try npx cache first, then local node_modules
    const findClaudeCodePath = () => {
        // Check npx cache (where npx @anthropic-ai/claude-code installs)
        const npxCachePath = join(homedir(), '.npm/_npx');
        try {
            const cacheEntries = readdirSync(npxCachePath);
            for (const entry of cacheEntries) {
                const cliPath = join(npxCachePath, entry, 'node_modules/@anthropic-ai/claude-code/cli.js');
                if (existsSync(cliPath)) {
                    return cliPath;
                }
            }
        }
        catch {
            // npx cache not found
        }
        // Check local node_modules
        const localPath = join(process.cwd(), 'node_modules/@anthropic-ai/claude-code/cli.js');
        if (existsSync(localPath)) {
            return localPath;
        }
        return undefined;
    };
    const claudeCodePath = findClaudeCodePath();
    if (claudeCodePath) {
        console.log(`[debug-agent] Found Claude Code at: ${claudeCodePath}`);
    }
    else {
        console.warn('[debug-agent] Claude Code not found, using default');
    }
    const options = {
        cwd: config.cwd,
        maxTurns: config.maxTurns,
        systemPrompt: DEBUG_AGENT_SYSTEM_PROMPT,
        mcpServers: {
            'interaction-tree': mcpServer,
        },
        allowedTools: DEBUG_AGENT_ALLOWED_TOOLS,
        permissionMode: 'bypassPermissions',
        allowDangerouslySkipPermissions: true,
        includePartialMessages: true,
        ...(claudeCodePath && { pathToClaudeCodeExecutable: claudeCodePath }),
    };
    options.model = config.model ?? DEFAULT_MODEL;
    console.log(`[debug-agent] Using model: ${options.model}`);
    if (config.resume) {
        options.resume = config.resume;
    }
    try {
        const textBlocks = [];
        let sessionId;
        for await (const message of query({ prompt: userMessage, options })) {
            // Capture session ID from the init message
            if (message.type === 'system' && message.subtype === 'init') {
                sessionId = message.session_id;
            }
            // Handle streaming partial messages
            if (message.type === 'stream_event') {
                const evt = message.event;
                if (evt.type === 'content_block_delta' && evt.delta?.type === 'text_delta') {
                    onEvent?.({ event: { kind: 'text_delta', text: evt.delta.text ?? '' } });
                }
                if (evt.type === 'content_block_start' && evt.content_block?.type === 'tool_use') {
                    onEvent?.({ event: {
                            kind: 'tool_call_start',
                            toolName: evt.content_block.name ?? '',
                            toolCallId: evt.content_block.id ?? ''
                        } });
                }
                if (evt.type === 'message_stop') {
                    onEvent?.({ event: { kind: 'message_complete' } });
                }
            }
            if (message.type === 'assistant') {
                for (const block of message.message.content) {
                    if (block.type === 'text') {
                        textBlocks.push(block.text);
                    }
                }
            }
            if (message.type === 'user') {
                for (const block of message.message.content) {
                    if (block.type === 'tool_result') {
                        const resultText = Array.isArray(block.content)
                            ? block.content.map((c) => c.type === 'text' ? c.text : '').join('')
                            : typeof block.content === 'string' ? block.content : undefined;
                        onEvent?.({ event: { kind: 'tool_call_end', toolName: '', toolCallId: block.tool_use_id, result: resultText } });
                    }
                }
            }
            if (message.type === 'result') {
                if (message.subtype !== 'success') {
                    const errorMsg = 'errors' in message ? message.errors.join(', ') : 'Unknown error';
                    onEvent?.({ event: { kind: 'error', message: errorMsg } });
                    return {
                        status: 'error',
                        error: errorMsg,
                        sessionId,
                    };
                }
            }
        }
        const allContent = textBlocks.join('\n');
        const finalContent = textBlocks.filter(t => t.trim()).pop() ?? '';
        const askContext = parseAskContext(allContent);
        if (askContext.isAskContext) {
            return {
                status: 'needs_context',
                question: askContext.question,
                suggestions: askContext.suggestions,
                sessionId,
            };
        }
        onEvent?.({ event: { kind: 'task_complete', summary: finalContent } });
        return {
            status: 'success',
            summary: finalContent,
            sessionId,
        };
    }
    catch (err) {
        console.error('[debug-agent] executeDebugAgent error:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        onEvent?.({ event: { kind: 'error', message: errorMsg } });
        return {
            status: 'error',
            error: errorMsg,
        };
    }
}
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
export class DebugAgentExecutor {
    config;
    /** Claude SDK session ID for resuming debug conversations */
    sdkSessionId;
    constructor(config) {
        this.config = config ?? {};
    }
    /** Get the current Claude SDK session ID */
    getSessionId() {
        return this.sdkSessionId;
    }
    /** Clear the session (start fresh debug investigation) */
    clearSession() {
        this.sdkSessionId = undefined;
    }
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
    async execute(options) {
        const { intent, sessionService, sessionManager, getSessionService, currentSessionId, projectPath, cwd, onEvent } = options;
        const config = getDefaultAgentConfig(cwd, this.config);
        // Resume existing session if we have one (multi-turn debugging)
        if (this.sdkSessionId) {
            config.resume = this.sdkSessionId;
        }
        const ctx = {
            sessionService,
            sessionManager,
            getSessionService,
            currentSessionId,
            projectPath,
        };
        const result = await executeDebugAgent(intent, config, ctx, onEvent);
        // Store the session ID for future resumption
        if (result.sessionId) {
            this.sdkSessionId = result.sessionId;
        }
        return result;
    }
}
//# sourceMappingURL=executor.js.map