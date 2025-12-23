/**
 * WebSocket server for fleeter-daemon.
 */
import type { MonitoringEvent } from './protocol.js';
import type { SessionManager } from '../session/index.js';
import type { FlutterProcessManager } from '../flutter/index.js';
import { SessionService } from '../session/service.js';
export interface DaemonServerConfig {
    port: number;
    host?: string;
}
export declare class DaemonServer {
    private wss;
    private clients;
    private sessionManager;
    private flutterManager;
    private sessionServices;
    private debugAgentExecutor;
    constructor(sessionManager: SessionManager, flutterManager: FlutterProcessManager);
    private initDebugAgent;
    registerSessionService(sessionId: string, service: SessionService): void;
    unregisterSessionService(sessionId: string): void;
    getSessionService(sessionId: string): SessionService | undefined;
    start(config: DaemonServerConfig): void;
    stop(): void;
    private handleMessage;
    private handleHello;
    private handleCommand;
    private handleDisconnect;
    private sendError;
    broadcastEvent(source: MonitoringEvent['source'], eventType: string, payload: unknown, sessionId?: string): void;
    getClientCount(): number;
}
//# sourceMappingURL=server.d.ts.map