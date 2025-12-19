/**
 * VM Service client for connecting to Flutter apps.
 * Implements JSON-RPC 2.0 over WebSocket.
 *
 * Subscribes to VM Service event streams (Extension, Debug) to receive
 * real-time notifications like Flutter.Frame and Flutter.Navigation,
 * similar to how Flutter DevTools gets widget tree updates.
 */

import { EventEmitter } from 'events';
import WebSocket from 'ws';
import { log } from '../logger.js';
import type {
  JsonRpcRequest,
  JsonRpcResponse,
  VMIsolateRef,
} from './protocol.js';
import type {
  InteractionTarget,
  InteractionResult,
  BatchStep,
  BatchResult,
  GetTreeOptions,
  LogEntry,
  RuntimeError,
  HotReloadResult,
  AppStatus,
} from './types.js';

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

/** Rate limiter for frame events (like DevTools at 5 FPS) */
class RateLimiter {
  private lastCall = 0;
  private pending = false;

  constructor(private fps: number) {}

  call(fn: () => void): void {
    const now = Date.now();
    const minInterval = 1000 / this.fps;

    if (now - this.lastCall >= minInterval) {
      this.lastCall = now;
      fn();
    } else if (!this.pending) {
      this.pending = true;
      setTimeout(() => {
        this.pending = false;
        this.lastCall = Date.now();
        fn();
      }, minInterval - (now - this.lastCall));
    }
  }
}

/** Error log with bounded size (like Dart MCP's ~5K token limit) */
class ErrorLog {
  private errors: RuntimeError[] = [];
  private maxSize = 20000; // ~5K tokens worth of characters
  private currentSize = 0;

  add(error: RuntimeError): void {
    const errorSize = (error.message?.length ?? 0) + (error.stackTrace?.length ?? 0);
    
    // Trim old errors if we'd exceed the limit
    while (this.currentSize + errorSize > this.maxSize && this.errors.length > 0) {
      const removed = this.errors.shift();
      if (removed) {
        this.currentSize -= (removed.message?.length ?? 0) + (removed.stackTrace?.length ?? 0);
      }
    }
    
    this.errors.push(error);
    this.currentSize += errorSize;
  }

  getAll(): RuntimeError[] {
    return [...this.errors];
  }

  clear(): void {
    this.errors = [];
    this.currentSize = 0;
  }
}

export class VMServiceClient extends EventEmitter {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pending = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();
  private isolateId: string | null = null;
  private uri: string | null = null;
  private onCloseCallbacks: Array<() => void> = [];
  private frameRateLimiter = new RateLimiter(5); // 5 FPS like DevTools
  private receivedNavigationEvent = false;
  private receivedReloadEvent = false;
  private errorLog = new ErrorLog();

  get isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  get connectionUri(): string | null {
    return this.uri;
  }

  /**
   * Connect to a Flutter app via VM service WebSocket.
   * Subscribes to Extension and Isolate event streams for real-time updates.
   */
  async connect(uri: string): Promise<void> {
    log.vm.info({ uri }, 'VM connect() called');
    log.vm.debug({ uri, uriLength: uri?.length, uriType: typeof uri }, 'VM connect() uri details');
    
    if (this.ws) {
      log.vm.debug('Existing WebSocket found, disconnecting first');
      await this.disconnect();
    }

    this.uri = uri;
    log.vm.debug({ uri }, 'Creating new WebSocket connection');

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(uri);
        log.vm.debug('WebSocket instance created, waiting for open event');
      } catch (err) {
        log.vm.error({ err, uri }, 'Failed to create WebSocket instance');
        reject(err);
        return;
      }

      this.ws.on('open', async () => {
        log.vm.info({ uri }, 'WebSocket opened successfully');
        try {
          log.vm.debug('Finding main isolate...');
          await this.findMainIsolate();
          log.vm.info({ isolateId: this.isolateId }, 'Found main isolate');
          
          log.vm.debug('Subscribing to streams...');
          await this.subscribeToStreams();
          log.vm.info({ uri, isolateId: this.isolateId }, 'VM client fully connected and subscribed to streams');
          resolve();
        } catch (err) {
          log.vm.error({ err, errMessage: err instanceof Error ? err.message : String(err) }, 'Failed to initialize VM client after WebSocket opened');
          // Close the socket if we can't find the isolate
          this.ws?.close();
          this.ws = null;
          this.uri = null;
          reject(err);
        }
      });

      this.ws.on('message', (data) => {
        this.handleMessage(data.toString());
      });

      this.ws.on('close', (code, reason) => {
        log.vm.warn({ code, reason: reason?.toString() }, 'WebSocket closed');
        this.handleClose();
      });

      this.ws.on('error', (err) => {
        log.vm.error({ err: err.message, uri }, 'WebSocket error event');
        reject(new Error(`WebSocket error: ${err.message}`));
      });
    });
  }

  /**
   * Subscribe to VM Service event streams for real-time updates.
   * Similar to how Flutter DevTools receives widget tree change notifications.
   * Also subscribes to Stderr for runtime error collection (like Dart MCP).
   */
  private async subscribeToStreams(): Promise<void> {
    log.vm.debug('Subscribing to VM Service streams');
    
    const streams = [
      { id: 'Extension', desc: 'Extension events (Flutter.Frame, Flutter.Navigation, Flutter.Error)' },
      { id: 'Isolate', desc: 'Isolate events (reload, restart)' },
      { id: 'Stderr', desc: 'Stderr output for error collection' },
    ];
    
    for (const stream of streams) {
      try {
        await this.callMethod('streamListen', { streamId: stream.id });
        log.vm.debug({ streamId: stream.id }, `Subscribed to ${stream.desc}`);
      } catch (err) {
        log.vm.warn({ err, streamId: stream.id }, `Failed to subscribe to ${stream.id} stream (may already be subscribed)`);
      }
    }
  }

  /**
   * Disconnect from the VM service.
   */
  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
      this.isolateId = null;
      this.uri = null;
    }
  }

  /**
   * Register a callback for when the connection closes.
   */
  onClose(callback: () => void): void {
    this.onCloseCallbacks.push(callback);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Interaction Tree Methods
  // ─────────────────────────────────────────────────────────────────────────────

  async getTree(options?: GetTreeOptions): Promise<InteractionTarget[]> {
    const result = await this.callExtension('ext.interaction_tree.getTree', {
      includeBounds: options?.includeBounds ?? false,
      includeWidgetType: options?.includeWidgetType ?? false,
      includeState: options?.includeState ?? false,
      summaryOnly: options?.summaryOnly ?? false,
    });
    return (result as { targets: InteractionTarget[] }).targets ?? [];
  }

  async execute(
    id: string,
    interaction: string,
    args?: Record<string, unknown>
  ): Promise<InteractionResult> {
    const rawResult = await this.callExtension('ext.interaction_tree.execute', {
      id,
      interaction,
      args,
    });
    
    log.vm.debug({ rawResult, hasTree: !!(rawResult as Record<string, unknown>)?.tree }, 'execute() raw result');
    
    const result = rawResult as InteractionResult;
    
    this.emit('interaction', { id, interaction, args, result });
    return result;
  }

  async tap(id: string): Promise<InteractionResult> {
    return this.execute(id, 'tap');
  }

  async doubleTap(id: string): Promise<InteractionResult> {
    return this.execute(id, 'doubleTap');
  }

  async longPress(id: string): Promise<InteractionResult> {
    return this.execute(id, 'longPress');
  }

  async enterText(id: string, text: string): Promise<InteractionResult> {
    return this.execute(id, 'enterText', { text });
  }

  async clearText(id: string): Promise<InteractionResult> {
    return this.execute(id, 'clearText');
  }

  async scroll(
    id: string,
    dx: number,
    dy: number
  ): Promise<InteractionResult> {
    return this.execute(id, 'scroll', { dx, dy });
  }

  async drag(id: string, dx: number, dy: number): Promise<InteractionResult> {
    return this.execute(id, 'drag', { dx, dy });
  }

  async scrollIntoView(
    id: string,
    alignment = 0
  ): Promise<InteractionResult> {
    return this.execute(id, 'scrollIntoView', { alignment });
  }

  async waitFor(
    id: string,
    condition: 'exists' | 'notExists' | 'visible' | 'notVisible' = 'exists',
    timeoutMs = 10000
  ): Promise<InteractionResult> {
    return this.execute(id, 'waitFor', { condition, timeoutMs });
  }

  async getState(id: string): Promise<Record<string, unknown>> {
    return (await this.callExtension('ext.interaction_tree.getState', {
      id,
    })) as Record<string, unknown>;
  }

  async executeAction(
    id: string,
    actionName: string,
    args?: Record<string, unknown>
  ): Promise<InteractionResult> {
    return this.execute(id, 'executeAction', { actionName, args });
  }

  async batch(steps: BatchStep[]): Promise<BatchResult> {
    return (await this.callExtension('ext.interaction_tree.batch', {
      steps,
    })) as BatchResult;
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Dart Tooling Methods (aligned with Dart MCP server approach)
  // ─────────────────────────────────────────────────────────────────────────────

  async hotReload(clearErrors = false): Promise<HotReloadResult> {
    if (clearErrors) {
      this.errorLog.clear();
    }
    
    try {
      // Try reloadSources first (same as Dart MCP)
      // This is the standard VM service method for hot reload
      const result = await this.callMethod('reloadSources', {
        isolateId: this.isolateId,
        force: false,
      }) as { success?: boolean; notices?: Array<{ message?: string }> };
      
      log.vm.debug({ result }, 'Hot reload result');
      
      if (result.success === false) {
        const notices = result.notices?.map(n => n.message).filter(Boolean).join('; ');
        return {
          success: false,
          error: notices || 'Reload failed - sources may have errors',
        };
      }
      
      // Trigger reassemble after reloadSources for Flutter to update widgets
      try {
        await this.callExtension('ext.flutter.reassemble', {});
      } catch {
        // Reassemble failure is non-fatal, reload still succeeded
        log.vm.warn('Reassemble failed after reload, but reload succeeded');
      }
      
      return { success: true, reloadedAt: new Date().toISOString() };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  async hotRestart(clearErrors = true): Promise<HotReloadResult> {
    // Always clear errors on restart (same as Dart MCP)
    if (clearErrors) {
      this.errorLog.clear();
    }
    
    try {
      // hotRestart is a service method, not an extension (called without isolateId prefix)
      const result = await this.callMethod('hotRestart', {
        isolateId: this.isolateId,
      }) as { type?: string };
      
      const success = result.type === 'Success' || result.type === undefined;
      return { 
        success, 
        restartedAt: new Date().toISOString(),
        error: success ? undefined : 'Restart returned non-success type',
      };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  async getLogs(_since?: string): Promise<LogEntry[]> {
    // Logs are collected via session manager from process stdout/stderr.
    // VM service log collection would require an ext.interaction_tree.getLogs
    // extension in the Flutter package. For now, this returns empty and
    // callers should use SessionManager.getLogs() instead.
    return [];
  }

  async getRuntimeErrors(clear = false): Promise<RuntimeError[]> {
    // Errors are collected from Flutter.Error extension events and Stderr stream
    // (same approach as Dart MCP server)
    const errors = this.errorLog.getAll();
    if (clear) {
      this.errorLog.clear();
    }
    return errors;
  }

  clearRuntimeErrors(): void {
    this.errorLog.clear();
  }

  async getStatus(): Promise<AppStatus> {
    return {
      connected: this.isConnected,
      isolateId: this.isolateId ?? undefined,
      appUri: this.uri ?? undefined,
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Internal Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async findMainIsolate(): Promise<void> {
    const vm = (await this.callMethod('getVM', {})) as {
      isolates: VMIsolateRef[];
    };

    // Find the main isolate (non-system isolate)
    const mainIsolate = vm.isolates.find((iso) => !iso.isSystemIsolate);
    if (!mainIsolate) {
      throw new Error('No main isolate found');
    }

    this.isolateId = mainIsolate.id;
  }

  private async callMethod(
    method: string,
    params: Record<string, unknown>
  ): Promise<unknown> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      log.vm.error({ 
        method, 
        hasWs: !!this.ws, 
        readyState: this.ws?.readyState,
        readyStateNames: { 0: 'CONNECTING', 1: 'OPEN', 2: 'CLOSING', 3: 'CLOSED' },
        uri: this.uri 
      }, 'callMethod failed: Not connected to VM service');
      throw new Error('Not connected to VM service');
    }

    const id = ++this.requestId;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    };

    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws!.send(JSON.stringify(request));

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error(`Request ${method} timed out`));
        }
      }, 30000);
    });
  }

  private async callExtension(
    method: string,
    args: Record<string, unknown>
  ): Promise<unknown> {
    if (!this.isolateId) {
      throw new Error('No isolate connected');
    }

    const result = await this.callMethod(method, {
      isolateId: this.isolateId,
      ...args,
    });

    return result;
  }

  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data) as JsonRpcResponse & {
        method?: string;
        params?: { streamId?: string; event?: { kind?: string; extensionKind?: string; extensionData?: unknown } };
      };

      // Handle stream events (notifications without id)
      if (message.method === 'streamNotify' && message.params) {
        log.vm.trace({ streamId: message.params.streamId, event: message.params.event }, 'Received stream notification');
        this.handleStreamEvent(message.params);
        return;
      }

      // Handle responses to our requests
      if (message.id !== undefined) {
        const pending = this.pending.get(message.id);
        if (pending) {
          this.pending.delete(message.id);
          if (message.error) {
            log.vm.debug({ id: message.id, error: message.error }, 'Request failed');
            pending.reject(
              new Error(message.error.message || 'Unknown error')
            );
          } else {
            pending.resolve(message.result);
          }
        }
      }
    } catch (err) {
      log.vm.warn({ err, data: data.substring(0, 200) }, 'Failed to parse message');
    }
  }

  /**
   * Handle incoming VM Service stream events.
   * Emits appropriate events for tree updates.
   * Collects runtime errors from Flutter.Error and Stderr streams (like Dart MCP).
   */
  private handleStreamEvent(params: { 
    streamId?: string; 
    event?: { 
      kind?: string; 
      extensionKind?: string; 
      extensionData?: unknown;
      bytes?: string; // Base64-encoded stderr bytes
      timestamp?: number;
    } 
  }): void {
    const { streamId, event } = params;
    if (!event) {
      log.vm.debug({ streamId }, 'Stream event with no event data');
      return;
    }

    // Extension events (Flutter.Frame, Flutter.Navigation, Flutter.Error, etc.)
    if (streamId === 'Extension') {
      const extensionKind = event.extensionKind;
      log.vm.debug({ extensionKind }, 'Extension event received');

      if (extensionKind === 'Flutter.Frame') {
        // Rate-limit frame events to 5 FPS
        if (this.receivedNavigationEvent || this.receivedReloadEvent) {
          log.vm.debug('Frame event after nav/reload');
          this.frameRateLimiter.call(() => {
            this.receivedNavigationEvent = false;
            this.receivedReloadEvent = false;
            this.emit('frame');
          });
        }
      } else if (extensionKind === 'Flutter.Navigation') {
        log.vm.info({ extensionData: event.extensionData }, 'Navigation event');
        this.receivedNavigationEvent = true;
        const route = (event.extensionData as { route?: string })?.route;
        this.emit('navigation', route);
      } else if (extensionKind === 'Flutter.FirstFrame') {
        log.vm.info('First frame event');
      } else if (extensionKind === 'Flutter.Error') {
        // Collect Flutter errors (same as Dart MCP)
        const errorData = event.extensionData as { 
          description?: string; 
          errorsSinceReload?: number;
          renderedErrorText?: string;
        } | undefined;
        
        const message = errorData?.renderedErrorText || 
                       errorData?.description || 
                       JSON.stringify(event.extensionData);
        
        log.vm.warn({ errorData }, 'Flutter error received');
        this.errorLog.add({
          message,
          timestamp: new Date().toISOString(),
        });
      } else {
        log.vm.trace({ extensionKind }, 'Unhandled extension event');
      }
    }

    // Stderr stream (runtime errors)
    if (streamId === 'Stderr' && event.bytes) {
      try {
        // Decode base64 stderr bytes (same as Dart MCP)
        const message = Buffer.from(event.bytes, 'base64').toString('utf-8').trim();
        if (message) {
          log.vm.debug({ message: message.substring(0, 100) }, 'Stderr received');
          this.errorLog.add({
            message,
            timestamp: new Date().toISOString(),
          });
        }
      } catch (err) {
        log.vm.warn({ err }, 'Failed to decode stderr bytes');
      }
    }

    // Isolate events (reload, restart)
    if (streamId === 'Isolate') {
      if (event.kind === 'IsolateReload') {
        this.receivedReloadEvent = true;
        this.emit('reload');
      }
    }
  }

  private handleClose(): void {
    this.ws = null;
    this.isolateId = null;

    // Reject all pending requests
    for (const [, { reject }] of this.pending) {
      reject(new Error('Connection closed'));
    }
    this.pending.clear();

    // Notify listeners (legacy callback style)
    for (const callback of this.onCloseCallbacks) {
      callback();
    }

    // Emit close event (new EventEmitter style)
    this.emit('close');
  }
}
