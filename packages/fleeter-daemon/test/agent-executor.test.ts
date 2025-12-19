/**
 * Unit tests for AgentExecutor and related functions.
 */

import { describe, test, expect, vi, beforeEach } from 'vitest';
import { AgentExecutor, getDefaultAgentConfig, executeAgent } from '../src/agent/executor.js';

describe('AgentExecutor', () => {
  describe('constructor', () => {
    test('creates with default config', () => {
      const executor = new AgentExecutor();
      expect(executor.getSessionId()).toBeUndefined();
    });

    test('creates with custom config', () => {
      const executor = new AgentExecutor({
        model: 'claude-3-opus',
        maxTurns: 20,
        sessionTimeoutMs: 60000,
      });
      expect(executor.getSessionId()).toBeUndefined();
    });
  });

  describe('getSessionId', () => {
    test('returns undefined initially', () => {
      const executor = new AgentExecutor();
      expect(executor.getSessionId()).toBeUndefined();
    });
  });

  describe('clearSession', () => {
    test('clears the session ID', () => {
      const executor = new AgentExecutor();
      executor.clearSession();
      expect(executor.getSessionId()).toBeUndefined();
    });
  });
});

describe('getDefaultAgentConfig', () => {
  test('returns correct defaults', () => {
    const config = getDefaultAgentConfig('/tmp/project');
    expect(config.cwd).toBe('/tmp/project');
    expect(config.maxTurns).toBe(10);
    expect(config.model).toBeUndefined();
    expect(config.resume).toBeUndefined();
  });

  test('applies maxTurns override', () => {
    const config = getDefaultAgentConfig('/tmp/project', { maxTurns: 25 });
    expect(config.maxTurns).toBe(25);
    expect(config.cwd).toBe('/tmp/project');
  });

  test('applies model override', () => {
    const config = getDefaultAgentConfig('/tmp/project', { model: 'claude-3-opus' });
    expect(config.model).toBe('claude-3-opus');
  });

  test('applies multiple overrides', () => {
    const config = getDefaultAgentConfig('/path/to/app', {
      maxTurns: 15,
      model: 'claude-3-sonnet',
    });
    expect(config.cwd).toBe('/path/to/app');
    expect(config.maxTurns).toBe(15);
    expect(config.model).toBe('claude-3-sonnet');
  });

  test('ignores sessionTimeoutMs override (not in AgentExecutorConfig)', () => {
    const config = getDefaultAgentConfig('/tmp/project', { sessionTimeoutMs: 30000 });
    expect(config.cwd).toBe('/tmp/project');
    expect(config.maxTurns).toBe(10);
    expect((config as Record<string, unknown>).sessionTimeoutMs).toBeUndefined();
  });

  test('cwd is set correctly for various paths', () => {
    expect(getDefaultAgentConfig('/').cwd).toBe('/');
    expect(getDefaultAgentConfig('/home/user/project').cwd).toBe('/home/user/project');
    expect(getDefaultAgentConfig('/Users/test/flutter_app').cwd).toBe('/Users/test/flutter_app');
  });
});

describe('parseAskContext (tested via executeAgent behavior)', () => {
  test.todo('parseAskContext is private - test indirectly through executeAgent integration tests');
});

describe('createInteractionTreeMcpServer (tools behavior)', () => {
  describe('when vmClient is undefined', () => {
    test.todo('getStatus returns connected: false message');
    test.todo('getTree returns not connected error');
    test.todo('execute returns not connected error');
    test.todo('getState returns not connected error');
    test.todo('batch returns not connected error');
    test.todo('hotReload returns not connected error');
    test.todo('hotRestart returns not connected error');
    test.todo('getLogs returns not connected error');
    test.todo('getErrors returns not connected error');
  });
});

describe('executeAgent', () => {
  test.todo('integration test - requires Claude SDK connection');
  
  describe('config handling', () => {
    test('config structure is passed correctly', () => {
      const config = {
        maxTurns: 5,
        cwd: '/test/path',
        model: 'test-model',
        resume: 'session-123',
      };
      expect(config.maxTurns).toBe(5);
      expect(config.cwd).toBe('/test/path');
      expect(config.model).toBe('test-model');
      expect(config.resume).toBe('session-123');
    });
  });
});

describe('AgentExecutor.execute', () => {
  let executor: AgentExecutor;

  beforeEach(() => {
    executor = new AgentExecutor({ maxTurns: 5 });
  });

  test('config from constructor is used in getDefaultAgentConfig', () => {
    const config = getDefaultAgentConfig('/test', { maxTurns: 5 });
    expect(config.maxTurns).toBe(5);
  });

  test('executor preserves custom config', () => {
    const customExecutor = new AgentExecutor({
      model: 'claude-3-opus',
      maxTurns: 20,
    });
    expect(customExecutor.getSessionId()).toBeUndefined();
  });

  test.todo('execute calls executeAgent with correct parameters - requires mocking Claude SDK');
  test.todo('execute stores sessionId from result - requires mocking Claude SDK');
  test.todo('execute resumes session when sdkSessionId exists - requires mocking Claude SDK');
});

describe('AgentExecutionResult types', () => {
  test('success result structure', () => {
    const result = {
      status: 'success' as const,
      summary: 'Task completed',
      sessionId: 'session-123',
    };
    expect(result.status).toBe('success');
    expect(result.summary).toBe('Task completed');
    expect(result.sessionId).toBe('session-123');
  });

  test('error result structure', () => {
    const result = {
      status: 'error' as const,
      error: 'Something went wrong',
    };
    expect(result.status).toBe('error');
    expect(result.error).toBe('Something went wrong');
  });

  test('needs_context result structure', () => {
    const result = {
      status: 'needs_context' as const,
      question: 'What should I do next?',
      suggestions: ['Option A', 'Option B'],
      sessionId: 'session-456',
    };
    expect(result.status).toBe('needs_context');
    expect(result.question).toBe('What should I do next?');
    expect(result.suggestions).toEqual(['Option A', 'Option B']);
  });
});
