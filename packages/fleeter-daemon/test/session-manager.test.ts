/**
 * Unit tests for SessionManager.
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { SessionManager } from '../src/session/manager.js';
import { MAX_SESSION_LOGS } from '../src/session/types.js';

describe('SessionManager', () => {
  let manager: SessionManager;

  beforeEach(() => {
    manager = new SessionManager();
  });

  describe('Session creation', () => {
    test('creates session with unique ID', () => {
      const session1 = manager.create({ name: 'test-1', projectPath: '/path/1' });
      const session2 = manager.create({ name: 'test-2', projectPath: '/path/2' });

      expect(session1.id).toBeDefined();
      expect(session2.id).toBeDefined();
      expect(session1.id).not.toBe(session2.id);
    });

    test('throws error for duplicate session names', () => {
      manager.create({ name: 'duplicate', projectPath: '/path' });

      expect(() => {
        manager.create({ name: 'duplicate', projectPath: '/path2' });
      }).toThrow('Session with name "duplicate" already exists');
    });

    test('sets initial status to not_running', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      expect(session.appStatus).toBe('not_running');
    });

    test('emits session:created event', () => {
      let emittedSession: unknown = null;
      manager.on('session:created', (s) => { emittedSession = s; });

      const session = manager.create({ name: 'test', projectPath: '/path' });

      expect(emittedSession).not.toBeNull();
      const info = emittedSession as { id: string; name: string; projectPath: string };
      expect(info.id).toBe(session.id);
      expect(info.name).toBe('test');
      expect(info.projectPath).toBe('/path');
    });

    test('sets createdAt and lastActiveAt timestamps', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      expect(session.createdAt).toBeDefined();
      expect(session.lastActiveAt).toBeDefined();
    });
  });

  describe('Session destruction', () => {
    test('destroys existing session', async () => {
      const session = manager.create({ name: 'to-destroy', projectPath: '/path' });

      await manager.destroy(session.id);

      expect(manager.get(session.id)).toBeUndefined();
    });

    test('throws error for non-existent session', async () => {
      let error: Error | null = null;
      try {
        await manager.destroy('non-existent');
      } catch (e) {
        error = e as Error;
      }
      expect(error).not.toBeNull();
      expect(error?.message).toBe('Session not found: non-existent');
    });

    test('emits session:destroying and session:destroyed events', async () => {
      let destroyingId: string | null = null;
      let destroyedId: string | null = null;
      manager.on('session:destroying', (id) => { destroyingId = id; });
      manager.on('session:destroyed', (id) => { destroyedId = id; });

      const session = manager.create({ name: 'test', projectPath: '/path' });
      await manager.destroy(session.id);

      expect(destroyingId).toBe(session.id);
      expect(destroyedId).toBe(session.id);
    });

    test('removes session from list', async () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });
      expect(manager.list()).toHaveLength(1);

      await manager.destroy(session.id);

      expect(manager.list()).toHaveLength(0);
    });

    test('can destroy session by name', async () => {
      manager.create({ name: 'by-name', projectPath: '/path' });

      await manager.destroy('by-name');

      expect(manager.get('by-name')).toBeUndefined();
    });
  });

  describe('Session lookup (get method)', () => {
    test('gets session by ID', () => {
      const created = manager.create({ name: 'test', projectPath: '/path' });

      const session = manager.get(created.id);

      expect(session).toBeDefined();
      expect(session?.name).toBe('test');
    });

    test('gets session by name', () => {
      manager.create({ name: 'find-by-name', projectPath: '/path' });

      const session = manager.get('find-by-name');

      expect(session).toBeDefined();
      expect(session?.name).toBe('find-by-name');
    });

    test('returns undefined for non-existent session', () => {
      expect(manager.get('does-not-exist')).toBeUndefined();
    });
  });

  describe('Client connection management', () => {
    test('connectClient adds client to session', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      const result = manager.connectClient(session.id, 'client-1');

      expect(result.connectedClients).toContain('client-1');
    });

    test('connectClient throws for non-existent session', () => {
      expect(() => {
        manager.connectClient('non-existent', 'client-1');
      }).toThrow('Session not found: non-existent');
    });

    test('connectClient emits session:client_connected event', () => {
      let emittedSessionId: string | null = null;
      let emittedClientId: string | null = null;
      manager.on('session:client_connected', (sid, cid) => { emittedSessionId = sid; emittedClientId = cid; });
      const session = manager.create({ name: 'test', projectPath: '/path' });

      manager.connectClient(session.id, 'client-1');

      expect(emittedSessionId).toBe(session.id);
      expect(emittedClientId).toBe('client-1');
    });

    test('disconnectClient removes client from session', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });
      manager.connectClient(session.id, 'client-1');

      manager.disconnectClient(session.id, 'client-1');

      const updated = manager.get(session.id);
      expect(updated?.connectedClients.has('client-1')).toBe(false);
    });

    test('disconnectClient emits session:client_disconnected event', () => {
      let emittedSessionId: string | null = null;
      let emittedClientId: string | null = null;
      manager.on('session:client_disconnected', (sid, cid) => { emittedSessionId = sid; emittedClientId = cid; });
      const session = manager.create({ name: 'test', projectPath: '/path' });
      manager.connectClient(session.id, 'client-1');

      manager.disconnectClient(session.id, 'client-1');

      expect(emittedSessionId).toBe(session.id);
      expect(emittedClientId).toBe('client-1');
    });

    test('disconnectClient handles non-existent session gracefully', () => {
      expect(() => {
        manager.disconnectClient('non-existent', 'client-1');
      }).not.toThrow();
    });

    test('disconnectClientFromAll removes client from all sessions', () => {
      const session1 = manager.create({ name: 'session-1', projectPath: '/path1' });
      const session2 = manager.create({ name: 'session-2', projectPath: '/path2' });
      manager.connectClient(session1.id, 'client-1');
      manager.connectClient(session2.id, 'client-1');

      manager.disconnectClientFromAll('client-1');

      expect(manager.get(session1.id)?.connectedClients.has('client-1')).toBe(false);
      expect(manager.get(session2.id)?.connectedClients.has('client-1')).toBe(false);
    });

    test('getClientSessions returns correct sessions', () => {
      const session1 = manager.create({ name: 'session-1', projectPath: '/path1' });
      const session2 = manager.create({ name: 'session-2', projectPath: '/path2' });
      manager.create({ name: 'session-3', projectPath: '/path3' });
      manager.connectClient(session1.id, 'client-1');
      manager.connectClient(session2.id, 'client-1');

      const clientSessions = manager.getClientSessions('client-1');

      expect(clientSessions).toHaveLength(2);
      expect(clientSessions.map(s => s.name)).toContain('session-1');
      expect(clientSessions.map(s => s.name)).toContain('session-2');
    });

    test('getClientSessions returns empty array for unconnected client', () => {
      manager.create({ name: 'test', projectPath: '/path' });

      const clientSessions = manager.getClientSessions('unconnected-client');

      expect(clientSessions).toHaveLength(0);
    });
  });

  describe('Status updates', () => {
    test('updateStatus changes status correctly', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      manager.updateStatus(session.id, 'running');

      expect(manager.get(session.id)?.appStatus).toBe('running');
    });

    test('updateStatus stores vmServiceUri and pid', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      manager.updateStatus(session.id, 'running', {
        vmServiceUri: 'ws://localhost:12345',
        pid: 42,
      });

      const updated = manager.get(session.id);
      expect(updated?.vmServiceUri).toBe('ws://localhost:12345');
      expect(updated?.pid).toBe(42);
    });

    test('emits session:status_changed event', () => {
      let emittedInfo: unknown = null;
      manager.on('session:status_changed', (info) => { emittedInfo = info; });
      const session = manager.create({ name: 'test', projectPath: '/path' });

      manager.updateStatus(session.id, 'running');

      expect(emittedInfo).not.toBeNull();
      const info = emittedInfo as { id: string; appStatus: string };
      expect(info.id).toBe(session.id);
      expect(info.appStatus).toBe('running');
    });

    test('handles non-existent session gracefully', () => {
      expect(() => {
        manager.updateStatus('non-existent', 'running');
      }).not.toThrow();
    });

    test('updates lastActiveAt timestamp', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });
      const originalLastActiveAt = manager.get(session.id)?.lastActiveAt;

      manager.updateStatus(session.id, 'running');

      const updated = manager.get(session.id);
      expect(updated?.lastActiveAt.getTime()).toBeGreaterThanOrEqual(originalLastActiveAt!.getTime());
    });
  });

  describe('Logs management', () => {
    test('addLog adds log to session', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      manager.addLog(session.id, 'Test log line');

      const logs = manager.getLogs(session.id);
      expect(logs).toContain('Test log line');
    });

    test('addLog handles non-existent session gracefully', () => {
      expect(() => {
        manager.addLog('non-existent', 'Test log');
      }).not.toThrow();
    });

    test('getLogs returns recent logs', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });
      manager.addLog(session.id, 'Line 1');
      manager.addLog(session.id, 'Line 2');
      manager.addLog(session.id, 'Line 3');

      const logs = manager.getLogs(session.id, 2);

      expect(logs).toHaveLength(2);
      expect(logs).toEqual(['Line 2', 'Line 3']);
    });

    test('getLogs returns empty array for non-existent session', () => {
      const logs = manager.getLogs('non-existent');
      expect(logs).toEqual([]);
    });

    test('clearLogs clears all logs', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });
      manager.addLog(session.id, 'Line 1');
      manager.addLog(session.id, 'Line 2');

      manager.clearLogs(session.id);

      expect(manager.getLogs(session.id)).toHaveLength(0);
    });

    test('clearLogs handles non-existent session gracefully', () => {
      expect(() => {
        manager.clearLogs('non-existent');
      }).not.toThrow();
    });

    test('logs are capped at MAX_SESSION_LOGS', () => {
      const session = manager.create({ name: 'test', projectPath: '/path' });

      for (let i = 0; i < MAX_SESSION_LOGS + 100; i++) {
        manager.addLog(session.id, `Line ${i}`);
      }

      const logs = manager.getLogs(session.id, MAX_SESSION_LOGS + 100);
      expect(logs).toHaveLength(MAX_SESSION_LOGS);
      expect(logs[0]).toBe(`Line 100`);
      expect(logs[logs.length - 1]).toBe(`Line ${MAX_SESSION_LOGS + 99}`);
    });
  });

  describe('list method', () => {
    test('returns all sessions as SessionInfo', () => {
      manager.create({ name: 'session-1', projectPath: '/path1' });
      manager.create({ name: 'session-2', projectPath: '/path2' });

      const list = manager.list();

      expect(list).toHaveLength(2);
      expect(list[0]).toHaveProperty('id');
      expect(list[0]).toHaveProperty('name');
      expect(list[0]).toHaveProperty('connectedClients');
      expect(Array.isArray(list[0].connectedClients)).toBe(true);
    });

    test('returns empty array when no sessions', () => {
      expect(manager.list()).toEqual([]);
    });
  });
});
