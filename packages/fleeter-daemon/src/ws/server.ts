/**
 * WebSocket server for fleeter-daemon.
 */

import { WebSocketServer, WebSocket } from 'ws';
import { v4 as uuidv4 } from 'uuid';
import type {
  ClientMessage,
  ConnectedClient,
  ClientHello,
  CommandMessage,
  CommandResponse,
  ServerHello,
  MonitoringEvent,
  AgentStreamEvent,
} from './protocol.js';
import { isClientHello, isCommandMessage, isAgentToolCall } from './protocol.js';
import type { SessionManager } from '../session/index.js';
import type { FlutterProcessManager } from '../flutter/index.js';
import type { BatchStep } from '../vm/index.js';
import { SessionService } from '../session/service.js';
import { log } from '../logger.js';

const DAEMON_VERSION = '0.1.0';

const NO_SESSION_ERROR = 'No session connected. Use list_sessions to see available sessions, then connect_session to connect to one.';

export interface DaemonServerConfig {
  port: number;
  host?: string;
}

export class DaemonServer {
  private wss: WebSocketServer | null = null;
  private clients = new Map<string, { client: ConnectedClient; ws: WebSocket }>();
  private sessionManager: SessionManager;
  private flutterManager: FlutterProcessManager;
  private sessionServices = new Map<string, SessionService>();

  constructor(
    sessionManager: SessionManager,
    flutterManager: FlutterProcessManager,
  ) {
    this.sessionManager = sessionManager;
    this.flutterManager = flutterManager;
  }

  registerSessionService(sessionId: string, service: SessionService): void {
    this.sessionServices.set(sessionId, service);
    log.ws.debug({ sessionId }, 'Registered session service');
  }

  unregisterSessionService(sessionId: string): void {
    this.sessionServices.delete(sessionId);
    log.ws.debug({ sessionId }, 'Unregistered session service');
  }

  getSessionService(sessionId: string): SessionService | undefined {
    return this.sessionServices.get(sessionId);
  }

  start(config: DaemonServerConfig): void {
    const { port, host = '127.0.0.1' } = config;

    if (this.wss) {
      log.ws.warn('WebSocket server already running');
      return;
    }

    this.wss = new WebSocketServer({ port, host });

    this.wss.on('connection', (ws) => {
      const tempId = uuidv4();
      log.ws.debug({ tempId }, 'New connection');

      ws.on('message', async (data) => {
        try {
          const msg = JSON.parse(data.toString()) as ClientMessage;
          await this.handleMessage(msg, ws, tempId);
        } catch (err) {
          log.ws.error({ err }, 'Error handling message');
          this.sendError(ws, 'unknown', err instanceof Error ? err.message : String(err));
        }
      });

      ws.on('close', () => this.handleDisconnect(tempId));
      ws.on('error', (err) => log.ws.error({ err: err.message }, 'WebSocket error'));
    });

    log.ws.info({ host, port }, 'WebSocket server listening');
  }

  stop(): void {
    if (this.wss) {
      for (const { client } of this.clients.values()) {
        this.sessionManager.disconnectClientFromAll(client.id);
      }
      this.wss.close();
      this.wss = null;
      log.ws.info('WebSocket server stopped');
    }
  }

  private async handleMessage(msg: ClientMessage, ws: WebSocket, tempId: string): Promise<void> {
    if (isClientHello(msg)) {
      await this.handleHello(msg, ws, tempId);
    } else if (isCommandMessage(msg)) {
      await this.handleCommand(msg, ws);
    } else if (isAgentToolCall(msg)) {
      this.sendError(ws, msg.id, 'Agent mode not yet implemented');
    }
  }

  private async handleHello(msg: ClientHello, ws: WebSocket, tempId: string): Promise<void> {
    this.clients.delete(tempId);

    const client: ConnectedClient = {
      id: msg.clientId,
      type: msg.clientType,
      connectedAt: new Date(),
      currentSessionId: null,
    };

    this.clients.set(msg.clientId, { client, ws });
    log.ws.info({ clientId: msg.clientId, clientType: msg.clientType }, 'Client registered');

    const sessions = this.sessionManager.list();
    const response: ServerHello = {
      type: 'hello_ack',
      daemonVersion: DAEMON_VERSION,
      sessions: sessions.map((s) => ({
        id: s.id,
        name: s.name,
        projectPath: s.projectPath,
        appStatus: s.appStatus,
      })),
    };

    ws.send(JSON.stringify(response));
  }

  private async handleCommand(msg: CommandMessage, ws: WebSocket): Promise<void> {
    const { id, clientId, action, data } = msg;

    const sendResponse = (response: Omit<CommandResponse, 'type' | 'id'>) => {
      ws.send(JSON.stringify({ type: 'command_response', id, ...response } satisfies CommandResponse));
    };

    try {
      switch (action) {
        case 'create_session': {
          const { name, projectPath } = data as { name: string; projectPath: string };
          if (!name || !projectPath) {
            sendResponse({ success: false, error: 'name and projectPath are required' });
            return;
          }
          const session = this.sessionManager.create({ name, projectPath });
          // Auto-connect the creating client to the new session
          this.sessionManager.connectClient(session.id, clientId);
          const clientEntry = this.clients.get(clientId);
          if (clientEntry) clientEntry.client.currentSessionId = session.id;
          sendResponse({ success: true, data: { session } });
          this.broadcastEvent('session', 'session.created', { session });
          break;
        }

        case 'destroy_session': {
          const { sessionId } = data as { sessionId: string };
          if (!sessionId) {
            sendResponse({ success: false, error: 'sessionId is required' });
            return;
          }
          await this.sessionManager.destroy(sessionId);
          sendResponse({ success: true });
          this.broadcastEvent('session', 'session.destroyed', { sessionId });
          break;
        }

        case 'list_sessions': {
          const sessions = this.sessionManager.list();
          sendResponse({ success: true, data: { sessions } });
          break;
        }

        case 'connect_session': {
          const { sessionId } = data as { sessionId: string };
          if (!sessionId) {
            sendResponse({ success: false, error: 'sessionId is required' });
            return;
          }
          const session = this.sessionManager.connectClient(sessionId, clientId);
          const clientEntry = this.clients.get(clientId);
          if (clientEntry) clientEntry.client.currentSessionId = session.id;
          
          // Send stored logs to the connecting client
          const storedLogs = this.sessionManager.getLogs(sessionId, 500);
          if (storedLogs.length > 0) {
            for (const line of storedLogs) {
              ws.send(JSON.stringify({
                type: 'event',
                ts: new Date().toISOString(),
                source: 'flutter',
                eventType: 'flutter.log',
                sessionId,
                payload: { line },
              }));
            }
          }
          
          // Send chat history to the connecting client
          const chatHistory = this.sessionManager.getChatHistory(sessionId);
          log.ws.info({ sessionId, chatHistoryCount: chatHistory.length, roles: chatHistory.map(m => m.role) }, 'Sending chat history on connect');

          sendResponse({ success: true, data: { session, chatHistory } });
          break;
        }

        case 'disconnect_session': {
          const { sessionId } = data as { sessionId: string };
          if (!sessionId) {
            sendResponse({ success: false, error: 'sessionId is required' });
            return;
          }
          this.sessionManager.disconnectClient(sessionId, clientId);
          const clientEntry = this.clients.get(clientId);
          if (clientEntry?.client.currentSessionId === sessionId) {
            clientEntry.client.currentSessionId = null;
          }
          sendResponse({ success: true });
          break;
        }

        case 'get_status': {
          const clientEntry = this.clients.get(clientId);
          const currentSessionId = clientEntry?.client.currentSessionId;
          const currentSession = currentSessionId ? this.sessionManager.get(currentSessionId) : null;

          sendResponse({
            success: true,
            data: {
              daemon: { version: DAEMON_VERSION, uptime: process.uptime() },
              client: clientEntry?.client ?? null,
              currentSession: currentSession ? this.sessionManager.list().find((s) => s.id === currentSession.id) : null,
              totalSessions: this.sessionManager.list().length,
              totalClients: this.clients.size,
            },
          });
          break;
        }

        case 'health_check': {
          sendResponse({ success: true, data: { healthy: true, version: DAEMON_VERSION } });
          break;
        }

        case 'run_app': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const session = this.sessionManager.get(sessionId);
          if (!session) {
            sendResponse({ success: false, error: 'Session not found' });
            return;
          }
          const options = data as { device?: string; flavor?: string; target?: string } | undefined;
          this.sessionManager.updateStatus(sessionId, 'starting');
          const flutterProcess = await this.flutterManager.runApp(sessionId, session.projectPath, options ?? {});
          this.sessionManager.updateStatus(sessionId, 'running', { pid: flutterProcess.pid });
          sendResponse({ success: true, data: { pid: flutterProcess.pid } });
          break;
        }

        case 'stop_app': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (service) {
            await service.stopApp();
          } else {
            await this.flutterManager.stopApp(sessionId);
          }
          sendResponse({ success: true });
          break;
        }

        case 'hot_reload': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const clearErrors = (data as { clearRuntimeErrors?: boolean })?.clearRuntimeErrors ?? false;
          const result = await service.hotReload(clearErrors);
          
          // Broadcast to all clients
          const clientType = clientEntry?.client.type ?? 'unknown';
          this.broadcastEvent('flutter', 'flutter.hot_reload', { result, clientType }, sessionId);
          
          sendResponse({ success: result.success, data: result, error: result.error });
          break;
        }

        case 'hot_restart': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const clearErrors = (data as { clearRuntimeErrors?: boolean })?.clearRuntimeErrors ?? true;
          const result = await service.hotRestart(clearErrors);
          
          // Broadcast to all clients
          const clientType = clientEntry?.client.type ?? 'unknown';
          this.broadcastEvent('flutter', 'flutter.hot_restart', { result, clientType }, sessionId);
          
          sendResponse({ success: result.success, data: result, error: result.error });
          break;
        }

        case 'get_tree': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'Not connected to VM' });
            return;
          }
          const options = data as { summaryOnly?: boolean } | undefined;
          const targets = await service.getTree({ includeWidgetType: true, summaryOnly: options?.summaryOnly });
          sendResponse({ success: true, data: { targets } });
          break;
        }

        case 'get_logs': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service) {
            sendResponse({ success: false, error: 'Session service not found' });
            return;
          }
          const options = data as { maxLines?: number } | undefined;
          const logs = service.getLogs(options?.maxLines);
          sendResponse({ success: true, data: { logs } });
          break;
        }

        case 'execute_interaction': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const { nodeId, interaction, args } = data as {
            nodeId: string;
            interaction: string;
            args?: Record<string, unknown>;
          };
          if (!nodeId || !interaction) {
            sendResponse({ success: false, error: 'nodeId and interaction are required' });
            return;
          }
          try {
            const result = await service.execute(nodeId, interaction, args);
            
            // Broadcast interaction to all clients so TUI sees actions from MCP
            const clientType = clientEntry?.client.type ?? 'unknown';
            this.broadcastEvent('interaction', 'interaction.executed', {
              nodeId,
              interaction,
              args,
              result,
              clientType,
            }, sessionId);
            
            sendResponse({ success: true, data: result });
          } catch (err) {
            sendResponse({
              success: false,
              error: err instanceof Error ? err.message : String(err),
            });
          }
          break;
        }

        case 'get_state': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const { nodeId } = data as { nodeId: string };
          if (!nodeId) {
            sendResponse({ success: false, error: 'nodeId is required' });
            return;
          }
          try {
            const state = await service.getState(nodeId);
            sendResponse({ success: true, data: { state } });
          } catch (err) {
            sendResponse({
              success: false,
              error: err instanceof Error ? err.message : String(err),
            });
          }
          break;
        }

        case 'batch': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const { steps } = data as { steps: BatchStep[] };
          if (!steps || !Array.isArray(steps)) {
            sendResponse({ success: false, error: 'steps array is required' });
            return;
          }
          try {
            const result = await service.batch(steps);
            sendResponse({ success: true, data: result });
          } catch (err) {
            sendResponse({
              success: false,
              error: err instanceof Error ? err.message : String(err),
            });
          }
          break;
        }

        case 'get_errors': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          if (!service?.isVmConnected) {
            sendResponse({ success: false, error: 'VM client not connected' });
            return;
          }
          const { clear } = data as { clear?: boolean } | undefined ?? {};
          try {
            const errors = await service.getRuntimeErrors();
            if (clear) {
              service.clearRuntimeErrors();
            }
            sendResponse({ success: true, data: { errors } });
          } catch (err) {
            sendResponse({
              success: false,
              error: err instanceof Error ? err.message : String(err),
            });
          }
          break;
        }

        case 'get_context': {
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          if (!sessionId) {
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const service = this.sessionServices.get(sessionId);
          const session = this.sessionManager.get(sessionId);
          if (!session) {
            sendResponse({ success: false, error: 'Session not found' });
            return;
          }

          const options = data as { maxLogs?: number; summaryTree?: boolean } | undefined;
          const maxLogs = options?.maxLogs ?? 50;
          const summaryTree = options?.summaryTree ?? true;

          try {
            // Gather all context in parallel where possible
            const context: {
              timestamp: string;
              sessionId: string;
              sessionName: string;
              appStatus: string;
              vmConnected: boolean;
              vmServiceUri?: string;
              tree: unknown[] | null;
              recentLogs: string[];
              runtimeErrors: unknown[];
            } = {
              timestamp: new Date().toISOString(),
              sessionId,
              sessionName: session.name,
              appStatus: session.appStatus,
              vmConnected: service?.isVmConnected ?? false,
              vmServiceUri: session.vmServiceUri ?? undefined,
              tree: null,
              recentLogs: service?.getLogs(maxLogs) ?? [],
              runtimeErrors: [],
            };

            // Only fetch tree and errors if VM is connected
            if (service?.isVmConnected) {
              const [tree, errors] = await Promise.all([
                service.getTree({ includeWidgetType: true, summaryOnly: summaryTree }).catch(() => null),
                service.getRuntimeErrors().catch(() => []),
              ]);
              context.tree = tree;
              context.runtimeErrors = errors;
            }

            log.ws.info({ sessionId, treeNodes: context.tree?.length ?? 0, logCount: context.recentLogs.length, errorCount: context.runtimeErrors.length }, 'get_context collected');
            sendResponse({ success: true, data: context });
          } catch (err) {
            sendResponse({
              success: false,
              error: err instanceof Error ? err.message : String(err),
            });
          }
          break;
        }

        case 'agent_message': {
          log.agent.debug({ clientId }, 'Received agent_message');
          const clientEntry = this.clients.get(clientId);
          const sessionId = clientEntry?.client.currentSessionId;
          log.agent.debug({ sessionId }, 'Session ID');
          if (!sessionId) {
            log.agent.warn('No session connected');
            sendResponse({ success: false, error: NO_SESSION_ERROR });
            return;
          }
          const session = this.sessionManager.get(sessionId);
          log.agent.debug({ sessionId: session?.id, hasAgent: !!session?.agent }, 'Session info');
          if (!session?.agent) {
            log.agent.warn({ sessionId }, 'No agent for session');
            sendResponse({ success: false, error: 'No agent for session' });
            return;
          }
          const { intent } = data as { intent: string };
          log.agent.info({ sessionId, intent }, 'Processing agent intent');

          // Store user message in chat history
          this.sessionManager.addChatMessage(sessionId, {
            role: 'user',
            content: { type: 'text', text: intent },
            timestamp: new Date().toISOString(),
          });

          // Broadcast user message to ALL connected clients so TUI sees messages from MCP
          const clientType = clientEntry?.client.type ?? 'unknown';
          log.agent.info({ sessionId, clientType, clientCount: this.clients.size }, 'Broadcasting user_message to all clients');
          
          const userMessageEvent: AgentStreamEvent = {
            type: 'agent_stream',
            id,
            sessionId,
            event: { kind: 'user_message', text: intent, clientId: clientType },
          };
          for (const [cid, { ws: clientWs, client }] of this.clients.entries()) {
            if (clientWs.readyState === WebSocket.OPEN) {
              log.agent.debug({ targetClientId: cid.slice(0, 8), targetClientType: client.type }, 'Sending user_message event');
              clientWs.send(JSON.stringify(userMessageEvent));
            }
          }

          const service = this.sessionServices.get(sessionId);
          log.agent.debug({ hasService: !!service, isVmConnected: service?.isVmConnected }, 'Session service status');

          // Track text and tool calls for chat history
          let textBuffer = '';
          const pendingToolCalls = new Map<string, { name: string; args: unknown; timestamp: string }>();

          const onEvent = (partialEvent: Omit<AgentStreamEvent, 'type' | 'id' | 'sessionId'>) => {
            log.agent.trace({ event: partialEvent.event }, 'Agent event');
            const evt = partialEvent.event;

            // Track text for chat history
            if (evt.kind === 'text_delta') {
              textBuffer += evt.text;
            }

            // Track tool calls for chat history
            if (evt.kind === 'tool_call_start') {
              pendingToolCalls.set(evt.toolCallId, {
                name: evt.toolName,
                args: {},
                timestamp: new Date().toISOString(),
              });
            }

            if (evt.kind === 'tool_call_end') {
              const pending = pendingToolCalls.get(evt.toolCallId);
              if (pending) {
                // Store completed tool call in chat history
                this.sessionManager.addChatMessage(sessionId, {
                  role: 'assistant',
                  content: {
                    type: 'tool_call',
                    name: pending.name,
                    args: pending.args,
                    output: evt.result,
                    status: 'success',
                  },
                  timestamp: pending.timestamp,
                });
                pendingToolCalls.delete(evt.toolCallId);
              }
            }

            // Store assistant text immediately when task completes (not after execute returns)
            // This ensures chat history is persisted even if TUI disconnects
            if (evt.kind === "task_complete") {
              log.agent.info({ hasText: !!textBuffer.trim(), textLen: textBuffer.length }, 'task_complete event');
              if (textBuffer.trim()) {
                this.sessionManager.addChatMessage(sessionId, {
                  role: "assistant",
                  content: { type: "text", text: textBuffer.trim() },
                  timestamp: new Date().toISOString(),
                });
                textBuffer = "";  // Clear so we dont double-save after execute()
              }
            }

            // Broadcast streaming events to ALL connected clients
            const streamEvent: AgentStreamEvent = {
              type: 'agent_stream',
              id,
              sessionId,
              event: partialEvent.event,
            };
            const eventJson = JSON.stringify(streamEvent);
            for (const { ws: clientWs } of this.clients.values()) {
              if (clientWs.readyState === WebSocket.OPEN) {
                clientWs.send(eventJson);
              }
            }
          };

          try {
            log.agent.debug({ sessionId }, 'Calling agent.execute');
            const result = await session.agent.execute({
              intent,
              sessionService: service,
              sessionManager: this.sessionManager,
              cwd: session.projectPath,
              onEvent,
            });
            log.agent.info({ sessionId, status: result.status }, 'Agent execute completed');

            // Store final assistant text only if not already saved on task_complete
            log.agent.info({ hasText: !!textBuffer.trim(), textLen: textBuffer.length }, 'after execute() - checking textBuffer');
            if (textBuffer.trim()) {
              log.agent.warn({ textLen: textBuffer.length }, 'SAVING TEXT AFTER EXECUTE - should not happen!');
              this.sessionManager.addChatMessage(sessionId, {
                role: 'assistant',
                content: { type: 'text', text: textBuffer.trim() },
                timestamp: new Date().toISOString(),
              });
            }

            const response = {
              type: 'agent_response',
              id,
              status: result.status,
              summary: result.summary,
              error: result.error,
              question: result.question,
              sdkSessionId: result.sessionId,
            };
            log.agent.debug({ responseType: response.type, status: response.status }, 'Sending response');
            ws.send(JSON.stringify(response));
          } catch (err) {
            log.agent.error({ err, sessionId }, 'Agent execute failed');
            ws.send(JSON.stringify({
              type: 'agent_response',
              id,
              status: 'error',
              error: err instanceof Error ? err.message : String(err),
            }));
          }
          break;
        }

        default:
          sendResponse({ success: false, error: `Unknown action: ${action}` });
      }
    } catch (err) {
      sendResponse({ success: false, error: err instanceof Error ? err.message : String(err) });
    }
  }

  private handleDisconnect(clientIdOrTemp: string): void {
    const clientEntry = this.clients.get(clientIdOrTemp);
    if (clientEntry) {
      log.ws.info({ clientId: clientEntry.client.id }, 'Client disconnected');
      this.sessionManager.disconnectClientFromAll(clientEntry.client.id);
      this.clients.delete(clientIdOrTemp);
    }
  }

  private sendError(ws: WebSocket, id: string, error: string): void {
    ws.send(JSON.stringify({ type: 'command_response', id, success: false, error } satisfies CommandResponse));
  }

  broadcastEvent(source: MonitoringEvent['source'], eventType: string, payload: unknown, sessionId?: string): void {
    const event: MonitoringEvent = {
      type: 'event',
      ts: new Date().toISOString(),
      source,
      eventType,
      sessionId,
      payload,
    };

    const message = JSON.stringify(event);
    for (const { ws } of this.clients.values()) {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(message);
      }
    }
  }

  getClientCount(): number {
    return this.clients.size;
  }
}
