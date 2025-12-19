/**
 * VM Service client for connecting to Flutter apps.
 * Implements JSON-RPC 2.0 over WebSocket.
 *
 * Subscribes to VM Service event streams (Extension, Debug) to receive
 * real-time notifications like Flutter.Frame and Flutter.Navigation,
 * similar to how Flutter DevTools gets widget tree updates.
 */
import { EventEmitter } from 'events';
import type { InteractionTarget, InteractionResult, BatchStep, BatchResult, GetTreeOptions, LogEntry, RuntimeError, HotReloadResult, AppStatus } from './types.js';
/** Interaction event data emitted after execute() */
export interface InteractionEvent {
    id: string;
    interaction: string;
    args?: Record<string, unknown>;
    result: InteractionResult;
}
/** Events emitted by VMServiceClient */
export interface VMServiceEvents {
    /** Fired on Flutter.Frame events (rate-limited) */
    frame: () => void;
    /** Fired on Flutter.Navigation events */
    navigation: (route?: string) => void;
    /** Fired on isolate reload (hot reload/restart) */
    reload: () => void;
    /** Fired when connection closes */
    close: () => void;
    /** Fired after an interaction is executed */
    interaction: (event: InteractionEvent) => void;
}
export declare class VMServiceClient extends EventEmitter {
    private ws;
    private requestId;
    private pending;
    private isolateId;
    private uri;
    private onCloseCallbacks;
    private frameRateLimiter;
    private receivedNavigationEvent;
    private receivedReloadEvent;
    private errorLog;
    get isConnected(): boolean;
    get connectionUri(): string | null;
    /**
     * Connect to a Flutter app via VM service WebSocket.
     * Subscribes to Extension and Isolate event streams for real-time updates.
     */
    connect(uri: string): Promise<void>;
    /**
     * Subscribe to VM Service event streams for real-time updates.
     * Similar to how Flutter DevTools receives widget tree change notifications.
     * Also subscribes to Stderr for runtime error collection (like Dart MCP).
     */
    private subscribeToStreams;
    /**
     * Disconnect from the VM service.
     */
    disconnect(): Promise<void>;
    /**
     * Register a callback for when the connection closes.
     */
    onClose(callback: () => void): void;
    getTree(options?: GetTreeOptions): Promise<InteractionTarget[]>;
    execute(id: string, interaction: string, args?: Record<string, unknown>): Promise<InteractionResult>;
    tap(id: string): Promise<InteractionResult>;
    doubleTap(id: string): Promise<InteractionResult>;
    longPress(id: string): Promise<InteractionResult>;
    enterText(id: string, text: string): Promise<InteractionResult>;
    clearText(id: string): Promise<InteractionResult>;
    scroll(id: string, dx: number, dy: number): Promise<InteractionResult>;
    drag(id: string, dx: number, dy: number): Promise<InteractionResult>;
    scrollIntoView(id: string, alignment?: number): Promise<InteractionResult>;
    waitFor(id: string, condition?: 'exists' | 'notExists' | 'visible' | 'notVisible', timeoutMs?: number): Promise<InteractionResult>;
    getState(id: string): Promise<Record<string, unknown>>;
    executeAction(id: string, actionName: string, args?: Record<string, unknown>): Promise<InteractionResult>;
    batch(steps: BatchStep[]): Promise<BatchResult>;
    hotReload(clearErrors?: boolean): Promise<HotReloadResult>;
    hotRestart(clearErrors?: boolean): Promise<HotReloadResult>;
    getLogs(_since?: string): Promise<LogEntry[]>;
    getRuntimeErrors(clear?: boolean): Promise<RuntimeError[]>;
    clearRuntimeErrors(): void;
    getStatus(): Promise<AppStatus>;
    private findMainIsolate;
    private callMethod;
    private callExtension;
    private handleMessage;
    /**
     * Handle incoming VM Service stream events.
     * Emits appropriate events for tree updates.
     * Collects runtime errors from Flutter.Error and Stderr streams (like Dart MCP).
     */
    private handleStreamEvent;
    private handleClose;
}
//# sourceMappingURL=client.d.ts.map