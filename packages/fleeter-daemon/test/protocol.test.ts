/**
 * WebSocket protocol validation tests.
 */

import { describe, test, expect } from 'vitest'
import {
  isClientHello,
  isCommandMessage,
  isAgentToolCall,
  type CommandAction,
} from '../src/ws/protocol.js'

describe('Protocol Type Guards', () => {
  describe('isClientHello', () => {
    test('returns true for valid ClientHello', () => {
      const msg = { type: 'hello', clientType: 'tui', clientId: 'test', version: '1.0' }
      expect(isClientHello(msg)).toBe(true)
    })

    test('returns true for mcp client type', () => {
      const msg = { type: 'hello', clientType: 'mcp', clientId: 'proxy-1', version: '2.0' }
      expect(isClientHello(msg)).toBe(true)
    })

    test('returns false for command message', () => {
      const msg = { type: 'command', id: '1', clientId: 'test', action: 'list_sessions' }
      expect(isClientHello(msg)).toBe(false)
    })

    test('returns false for agent_tool_call message', () => {
      const msg = { type: 'agent_tool_call', id: '1', clientId: 'test', sessionId: 's1', intent: 'tap button' }
      expect(isClientHello(msg)).toBe(false)
    })

    test('returns false for null', () => {
      expect(isClientHello(null)).toBe(false)
    })

    test('returns false for undefined', () => {
      expect(isClientHello(undefined)).toBe(false)
    })

    test('returns false for non-objects', () => {
      expect(isClientHello('hello')).toBe(false)
      expect(isClientHello(123)).toBe(false)
      expect(isClientHello(true)).toBe(false)
      expect(isClientHello([])).toBe(false)
    })

    test('returns false for object with wrong type field', () => {
      const msg = { type: 'goodbye', clientType: 'tui', clientId: 'test', version: '1.0' }
      expect(isClientHello(msg)).toBe(false)
    })

    test('returns false for empty object', () => {
      expect(isClientHello({})).toBe(false)
    })

    test('returns true even with extra fields', () => {
      const msg = { type: 'hello', clientType: 'tui', clientId: 'test', version: '1.0', extra: 'field' }
      expect(isClientHello(msg)).toBe(true)
    })
  })

  describe('isCommandMessage', () => {
    test('returns true for valid CommandMessage', () => {
      const msg = { type: 'command', id: '1', clientId: 'test', action: 'list_sessions' }
      expect(isCommandMessage(msg)).toBe(true)
    })

    test('returns true for CommandMessage with data', () => {
      const msg = {
        type: 'command',
        id: '2',
        clientId: 'test',
        action: 'create_session',
        data: { name: 'my-session', projectPath: '/path/to/project' },
      }
      expect(isCommandMessage(msg)).toBe(true)
    })

    test('returns false for hello message', () => {
      const msg = { type: 'hello', clientType: 'tui', clientId: 'test', version: '1.0' }
      expect(isCommandMessage(msg)).toBe(false)
    })

    test('returns false for agent_tool_call message', () => {
      const msg = { type: 'agent_tool_call', id: '1', clientId: 'test', sessionId: 's1', intent: 'tap button' }
      expect(isCommandMessage(msg)).toBe(false)
    })

    test('returns false for null', () => {
      expect(isCommandMessage(null)).toBe(false)
    })

    test('returns false for undefined', () => {
      expect(isCommandMessage(undefined)).toBe(false)
    })

    test('returns false for non-objects', () => {
      expect(isCommandMessage('command')).toBe(false)
      expect(isCommandMessage(42)).toBe(false)
      expect(isCommandMessage(false)).toBe(false)
    })

    test('returns false for object with wrong type field', () => {
      const msg = { type: 'request', id: '1', clientId: 'test', action: 'list_sessions' }
      expect(isCommandMessage(msg)).toBe(false)
    })

    test('returns false for empty object', () => {
      expect(isCommandMessage({})).toBe(false)
    })
  })

  describe('isAgentToolCall', () => {
    test('returns true for valid AgentToolCall', () => {
      const msg = {
        type: 'agent_tool_call',
        id: '1',
        clientId: 'test',
        sessionId: 'session-1',
        intent: 'tap the submit button',
      }
      expect(isAgentToolCall(msg)).toBe(true)
    })

    test('returns true for AgentToolCall with optional fields', () => {
      const msg = {
        type: 'agent_tool_call',
        id: '2',
        clientId: 'test',
        sessionId: 'session-1',
        intent: 'enter text',
        context: ['previous action 1', 'previous action 2'],
        conversationId: 'conv-123',
      }
      expect(isAgentToolCall(msg)).toBe(true)
    })

    test('returns false for hello message', () => {
      const msg = { type: 'hello', clientType: 'tui', clientId: 'test', version: '1.0' }
      expect(isAgentToolCall(msg)).toBe(false)
    })

    test('returns false for command message', () => {
      const msg = { type: 'command', id: '1', clientId: 'test', action: 'list_sessions' }
      expect(isAgentToolCall(msg)).toBe(false)
    })

    test('returns false for null', () => {
      expect(isAgentToolCall(null)).toBe(false)
    })

    test('returns false for undefined', () => {
      expect(isAgentToolCall(undefined)).toBe(false)
    })

    test('returns false for non-objects', () => {
      expect(isAgentToolCall('agent_tool_call')).toBe(false)
      expect(isAgentToolCall(999)).toBe(false)
    })

    test('returns false for object with wrong type field', () => {
      const msg = { type: 'tool_call', id: '1', clientId: 'test', sessionId: 's1', intent: 'test' }
      expect(isAgentToolCall(msg)).toBe(false)
    })

    test('returns false for empty object', () => {
      expect(isAgentToolCall({})).toBe(false)
    })
  })

  describe('CommandAction enum coverage', () => {
    const validActions: CommandAction[] = [
      'create_session',
      'destroy_session',
      'list_sessions',
      'connect_session',
      'disconnect_session',
      'run_app',
      'stop_app',
      'hot_reload',
      'hot_restart',
      'get_tree',
      'execute_interaction',
      'get_logs',
      'agent_message',
      'get_status',
      'health_check',
    ]

    test('all action types are valid strings', () => {
      for (const action of validActions) {
        expect(typeof action).toBe('string')
        expect(action.length).toBeGreaterThan(0)
      }
    })

    test('session management actions exist', () => {
      const sessionActions: CommandAction[] = [
        'create_session',
        'destroy_session',
        'list_sessions',
        'connect_session',
        'disconnect_session',
      ]
      for (const action of sessionActions) {
        expect(validActions).toContain(action)
      }
    })

    test('app lifecycle actions exist', () => {
      const appActions: CommandAction[] = ['run_app', 'stop_app', 'hot_reload', 'hot_restart']
      for (const action of appActions) {
        expect(validActions).toContain(action)
      }
    })

    test('interaction tree actions exist', () => {
      const treeActions: CommandAction[] = ['get_tree', 'execute_interaction']
      for (const action of treeActions) {
        expect(validActions).toContain(action)
      }
    })

    test('status actions exist', () => {
      const statusActions: CommandAction[] = ['get_status', 'health_check']
      for (const action of statusActions) {
        expect(validActions).toContain(action)
      }
    })
  })
})
