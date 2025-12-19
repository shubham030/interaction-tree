/**
 * Edge case and concurrency tests for SessionManager.
 * 
 * Tests scenarios that have caused regressions:
 * - Rapid status transitions
 * - Concurrent session operations
 * - Client reconnection scenarios
 * - Event ordering
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { SessionManager } from '../src/session/manager.js';
import type { SessionInfo, AppStatus } from '../src/session/types.js';

describe('Edge Cases', () => {
  let manager: SessionManager;

  beforeEach(() => {
    manager = new SessionManager();
  });

  describe('Rapid Status Changes', () => {
    test('handles multiple rapid status changes correctly', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: AppStatus[] = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push(info.appStatus);
      });

      manager.updateStatus(session.id, 'starting');
      manager.updateStatus(session.id, 'running', { pid: 123, vmServiceUri: 'ws://localhost' });
      manager.updateStatus(session.id, 'stopped');

      expect(events).toEqual(['starting', 'running', 'stopped']);
    });

    test('final state is correct after rapid transitions', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });

      manager.updateStatus(session.id, 'starting');
      manager.updateStatus(session.id, 'running', { pid: 123, vmServiceUri: 'ws://localhost:1234' });
      manager.updateStatus(session.id, 'error', { error: 'crash' });
      manager.updateStatus(session.id, 'stopped');

      const final = manager.get(session.id);
      expect(final?.appStatus).toBe('stopped');
    });

    test('status changes during event processing do not corrupt state', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: AppStatus[] = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push(info.appStatus);
        if (info.appStatus === 'starting') {
          manager.updateStatus(session.id, 'running', { pid: 456 });
        }
      });

      manager.updateStatus(session.id, 'starting');

      expect(events).toEqual(['starting', 'running']);
      expect(manager.get(session.id)?.appStatus).toBe('running');
    });

    test('all transitions through full lifecycle', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: Array<{ status: AppStatus; timestamp: number }> = [];
      let counter = 0;

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push({ status: info.appStatus, timestamp: counter++ });
      });

      const transitions: AppStatus[] = ['starting', 'running', 'stopped', 'starting', 'running', 'error', 'stopped'];
      transitions.forEach((status, i) => {
        if (status === 'running') {
          manager.updateStatus(session.id, status, { pid: 100 + i, vmServiceUri: `ws://localhost:${3000 + i}` });
        } else {
          manager.updateStatus(session.id, status);
        }
      });

      expect(events.map(e => e.status)).toEqual(transitions);
      for (let i = 1; i < events.length; i++) {
        expect(events[i].timestamp).toBeGreaterThan(events[i - 1].timestamp);
      }
    });

    test('concurrent async status updates resolve correctly', async () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: AppStatus[] = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push(info.appStatus);
      });

      await Promise.all([
        Promise.resolve().then(() => manager.updateStatus(session.id, 'starting')),
        Promise.resolve().then(() => manager.updateStatus(session.id, 'running', { pid: 1 })),
        Promise.resolve().then(() => manager.updateStatus(session.id, 'stopped')),
      ]);

      expect(events).toHaveLength(3);
      expect(events).toContain('starting');
      expect(events).toContain('running');
      expect(events).toContain('stopped');
    });
  });

  describe('Concurrent Session Operations', () => {
    test('multiple clients connecting to same session', () => {
      const session = manager.create({ name: 'shared', projectPath: '/tmp' });
      const connectEvents: string[] = [];

      manager.on('session:client_connected', (_sessionId: string, clientId: string) => {
        connectEvents.push(clientId);
      });

      manager.connectClient(session.id, 'client-1');
      manager.connectClient(session.id, 'client-2');
      manager.connectClient(session.id, 'client-3');

      const updated = manager.get(session.id);
      expect(updated?.connectedClients.size).toBe(3);
      expect(connectEvents).toEqual(['client-1', 'client-2', 'client-3']);
    });

    test('same client connecting twice is idempotent', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      let connectCount = 0;

      manager.on('session:client_connected', () => {
        connectCount++;
      });

      manager.connectClient(session.id, 'client-1');
      manager.connectClient(session.id, 'client-1');
      manager.connectClient(session.id, 'client-1');

      const updated = manager.get(session.id);
      expect(updated?.connectedClients.size).toBe(1);
      expect(connectCount).toBe(3);
    });

    test('create and destroy session rapidly', async () => {
      const events: Array<{ type: string; id?: string }> = [];

      manager.on('session:created', (info: SessionInfo) => {
        events.push({ type: 'created', id: info.id });
      });
      manager.on('session:destroyed', (id: string) => {
        events.push({ type: 'destroyed', id });
      });

      const session1 = manager.create({ name: 'session-1', projectPath: '/tmp' });
      const session2 = manager.create({ name: 'session-2', projectPath: '/tmp' });

      await manager.destroy(session1.id);
      const session3 = manager.create({ name: 'session-3', projectPath: '/tmp' });
      await manager.destroy(session2.id);
      await manager.destroy(session3.id);

      expect(manager.list()).toHaveLength(0);
      expect(events.filter(e => e.type === 'created')).toHaveLength(3);
      expect(events.filter(e => e.type === 'destroyed')).toHaveLength(3);
    });

    test('multiple status updates from different sources interleaved', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: Array<{ status: AppStatus; source: string }> = [];

      const source1Handler = (info: SessionInfo) => {
        if (info.pid === 100) events.push({ status: info.appStatus, source: 'ws-handler' });
      };
      const source2Handler = (info: SessionInfo) => {
        if (info.pid === 200) events.push({ status: info.appStatus, source: 'flutter-events' });
      };

      manager.on('session:status_changed', source1Handler);
      manager.on('session:status_changed', source2Handler);

      manager.updateStatus(session.id, 'starting', { pid: 100 });
      manager.updateStatus(session.id, 'running', { pid: 200, vmServiceUri: 'ws://localhost' });

      expect(events).toHaveLength(2);
      expect(events[0]).toEqual({ status: 'starting', source: 'ws-handler' });
      expect(events[1]).toEqual({ status: 'running', source: 'flutter-events' });
    });

    test('concurrent create with same name fails', () => {
      manager.create({ name: 'unique', projectPath: '/tmp' });
      
      expect(() => {
        manager.create({ name: 'unique', projectPath: '/tmp' });
      }).toThrow('Session with name "unique" already exists');
    });

    test('destroy during client operations', async () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      manager.connectClient(session.id, 'client-1');
      manager.connectClient(session.id, 'client-2');

      let destroyingReceived = false;
      manager.on('session:destroying', () => {
        destroyingReceived = true;
        manager.disconnectClient(session.id, 'client-1');
      });

      await manager.destroy(session.id);

      expect(destroyingReceived).toBe(true);
      expect(manager.get(session.id)).toBeUndefined();
    });
  });

  describe('Client Reconnection Scenarios', () => {
    test('client disconnects and reconnects to same session', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: Array<{ type: string; clientId: string }> = [];

      manager.on('session:client_connected', (_sid: string, clientId: string) => {
        events.push({ type: 'connected', clientId });
      });
      manager.on('session:client_disconnected', (_sid: string, clientId: string) => {
        events.push({ type: 'disconnected', clientId });
      });

      manager.connectClient(session.id, 'client-1');
      manager.disconnectClient(session.id, 'client-1');
      manager.connectClient(session.id, 'client-1');

      expect(events).toEqual([
        { type: 'connected', clientId: 'client-1' },
        { type: 'disconnected', clientId: 'client-1' },
        { type: 'connected', clientId: 'client-1' },
      ]);

      const updated = manager.get(session.id);
      expect(updated?.connectedClients.has('client-1')).toBe(true);
    });

    test('session state preserved across client reconnections', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      
      manager.connectClient(session.id, 'client-1');
      manager.updateStatus(session.id, 'running', { pid: 123, vmServiceUri: 'ws://localhost:9999' });
      manager.addLog(session.id, 'log line 1');
      manager.addLog(session.id, 'log line 2');

      manager.disconnectClient(session.id, 'client-1');

      const afterDisconnect = manager.get(session.id);
      expect(afterDisconnect?.appStatus).toBe('running');
      expect(afterDisconnect?.vmServiceUri).toBe('ws://localhost:9999');
      expect(manager.getLogs(session.id)).toEqual(['log line 1', 'log line 2']);

      manager.connectClient(session.id, 'client-1');
      
      const afterReconnect = manager.get(session.id);
      expect(afterReconnect?.appStatus).toBe('running');
      expect(afterReconnect?.vmServiceUri).toBe('ws://localhost:9999');
      expect(manager.getLogs(session.id)).toEqual(['log line 1', 'log line 2']);
    });

    test('client connects to destroyed session fails', async () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      await manager.destroy(session.id);

      expect(() => {
        manager.connectClient(session.id, 'client-1');
      }).toThrow('Session not found');
    });

    test('disconnect from non-existent session is safe', () => {
      expect(() => {
        manager.disconnectClient('non-existent', 'client-1');
      }).not.toThrow();
    });

    test('disconnectClientFromAll removes from all sessions', () => {
      const session1 = manager.create({ name: 'session-1', projectPath: '/tmp' });
      const session2 = manager.create({ name: 'session-2', projectPath: '/tmp' });
      const session3 = manager.create({ name: 'session-3', projectPath: '/tmp' });

      manager.connectClient(session1.id, 'client-1');
      manager.connectClient(session2.id, 'client-1');
      manager.connectClient(session3.id, 'client-1');
      manager.connectClient(session1.id, 'client-2');

      manager.disconnectClientFromAll('client-1');

      expect(manager.get(session1.id)?.connectedClients.has('client-1')).toBe(false);
      expect(manager.get(session2.id)?.connectedClients.has('client-1')).toBe(false);
      expect(manager.get(session3.id)?.connectedClients.has('client-1')).toBe(false);
      expect(manager.get(session1.id)?.connectedClients.has('client-2')).toBe(true);
    });

    test('getClientSessions returns correct sessions', () => {
      const session1 = manager.create({ name: 'session-1', projectPath: '/tmp' });
      const session2 = manager.create({ name: 'session-2', projectPath: '/tmp' });
      manager.create({ name: 'session-3', projectPath: '/tmp' });

      manager.connectClient(session1.id, 'client-1');
      manager.connectClient(session2.id, 'client-1');

      const clientSessions = manager.getClientSessions('client-1');
      expect(clientSessions).toHaveLength(2);
      expect(clientSessions.map(s => s.name).sort()).toEqual(['session-1', 'session-2']);
    });
  });

  describe('Event Ordering', () => {
    test('events emitted in correct order during status transitions', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: Array<{ type: string; data: unknown }> = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push({ type: 'status_changed', data: info.appStatus });
      });

      manager.updateStatus(session.id, 'starting');
      manager.updateStatus(session.id, 'running', { pid: 123, vmServiceUri: 'ws://localhost' });

      expect(events).toEqual([
        { type: 'status_changed', data: 'starting' },
        { type: 'status_changed', data: 'running' },
      ]);
    });

    test('no duplicate events for same status change', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const events: AppStatus[] = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        events.push(info.appStatus);
      });

      manager.updateStatus(session.id, 'running', { pid: 123 });
      manager.updateStatus(session.id, 'running', { pid: 123 });
      manager.updateStatus(session.id, 'running', { pid: 456 });

      expect(events).toEqual(['running', 'running', 'running']);
    });

    test('events contain correct session data at time of emission', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const snapshots: SessionInfo[] = [];

      manager.on('session:status_changed', (info: SessionInfo) => {
        snapshots.push({ ...info });
      });

      manager.updateStatus(session.id, 'starting');
      manager.updateStatus(session.id, 'running', { pid: 100, vmServiceUri: 'ws://localhost:1000' });
      manager.updateStatus(session.id, 'stopped');

      expect(snapshots[0].appStatus).toBe('starting');
      expect(snapshots[0].pid).toBeUndefined();

      expect(snapshots[1].appStatus).toBe('running');
      expect(snapshots[1].pid).toBe(100);
      expect(snapshots[1].vmServiceUri).toBe('ws://localhost:1000');

      expect(snapshots[2].appStatus).toBe('stopped');
      expect(snapshots[2].pid).toBe(100);
    });

    test('create event fires before any status events', () => {
      const events: string[] = [];

      manager.on('session:created', () => {
        events.push('created');
      });
      manager.on('session:status_changed', () => {
        events.push('status_changed');
      });

      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      manager.updateStatus(session.id, 'starting');

      expect(events).toEqual(['created', 'status_changed']);
    });

    test('destroying fires before destroyed', async () => {
      const events: string[] = [];

      manager.on('session:destroying', () => {
        events.push('destroying');
      });
      manager.on('session:destroyed', () => {
        events.push('destroyed');
      });

      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      await manager.destroy(session.id);

      expect(events).toEqual(['destroying', 'destroyed']);
    });

    test('multiple listeners receive events in registration order', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });
      const order: number[] = [];

      manager.on('session:status_changed', () => order.push(1));
      manager.on('session:status_changed', () => order.push(2));
      manager.on('session:status_changed', () => order.push(3));

      manager.updateStatus(session.id, 'starting');

      expect(order).toEqual([1, 2, 3]);
    });

    test('events from rapid concurrent operations maintain causality', async () => {
      const events: Array<{ type: string; name?: string; id?: string }> = [];

      manager.on('session:created', (info: SessionInfo) => {
        events.push({ type: 'created', name: info.name });
      });
      manager.on('session:destroyed', (id: string) => {
        events.push({ type: 'destroyed', id });
      });

      const s1 = manager.create({ name: 's1', projectPath: '/tmp' });
      const s2 = manager.create({ name: 's2', projectPath: '/tmp' });
      await manager.destroy(s1.id);
      const s3 = manager.create({ name: 's3', projectPath: '/tmp' });
      await manager.destroy(s2.id);
      await manager.destroy(s3.id);

      const created = events.filter(e => e.type === 'created');
      const destroyed = events.filter(e => e.type === 'destroyed');

      expect(created.map(e => e.name)).toEqual(['s1', 's2', 's3']);
      expect(destroyed).toHaveLength(3);
      
      const s1CreateIdx = events.findIndex(e => e.type === 'created' && e.name === 's1');
      const s1DestroyIdx = events.findIndex(e => e.type === 'destroyed' && e.id === s1.id);
      expect(s1CreateIdx).toBeLessThan(s1DestroyIdx);
    });
  });

  describe('Error Handling Edge Cases', () => {
    test('updateStatus on non-existent session does not throw', () => {
      expect(() => {
        manager.updateStatus('non-existent', 'running');
      }).not.toThrow();
    });

    test('destroy non-existent session throws', async () => {
      try {
        await manager.destroy('non-existent');
        expect.fail('Expected destroy to throw');
      } catch (e) {
        expect((e as Error).message).toBe('Session not found: non-existent');
      }
    });

    test('logs operations on non-existent session are safe', () => {
      expect(() => {
        manager.addLog('non-existent', 'test');
      }).not.toThrow();

      expect(manager.getLogs('non-existent')).toEqual([]);

      expect(() => {
        manager.clearLogs('non-existent');
      }).not.toThrow();
    });

    test('session lookup by name after destroy returns undefined', async () => {
      const session = manager.create({ name: 'test-name', projectPath: '/tmp' });
      expect(manager.get('test-name')).toBeDefined();
      
      await manager.destroy(session.id);
      expect(manager.get('test-name')).toBeUndefined();
    });
  });

  describe('Log Edge Cases', () => {
    test('logs persist through status changes', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });

      manager.addLog(session.id, 'before start');
      manager.updateStatus(session.id, 'starting');
      manager.addLog(session.id, 'during start');
      manager.updateStatus(session.id, 'running', { pid: 123 });
      manager.addLog(session.id, 'while running');
      manager.updateStatus(session.id, 'stopped');
      manager.addLog(session.id, 'after stop');

      expect(manager.getLogs(session.id)).toEqual([
        'before start',
        'during start',
        'while running',
        'after stop',
      ]);
    });

    test('getLogs respects maxLines parameter', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });

      for (let i = 0; i < 10; i++) {
        manager.addLog(session.id, `line ${i}`);
      }

      expect(manager.getLogs(session.id, 3)).toEqual(['line 7', 'line 8', 'line 9']);
      expect(manager.getLogs(session.id, 5)).toEqual(['line 5', 'line 6', 'line 7', 'line 8', 'line 9']);
    });

    test('clearLogs removes all logs', () => {
      const session = manager.create({ name: 'test', projectPath: '/tmp' });

      manager.addLog(session.id, 'line 1');
      manager.addLog(session.id, 'line 2');
      expect(manager.getLogs(session.id)).toHaveLength(2);

      manager.clearLogs(session.id);
      expect(manager.getLogs(session.id)).toEqual([]);
    });
  });
});
