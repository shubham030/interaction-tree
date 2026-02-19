/**
 * Toolbox Generator - Creates executable scripts for the Debug Agent.
 *
 * These scripts communicate with the daemon via WebSocket to execute
 * commands like list_sessions, create_session, get_tree, etc.
 *
 * The debug agent uses these via the Amp SDK's `toolbox` option.
 */
import { mkdirSync, writeFileSync, chmodSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
/** All tools available to the debug agent */
const TOOLS = [
    // Session Management
    {
        name: 'list_sessions',
        description: 'List all available Flutter debug sessions. Returns session IDs, names, project paths, and status.',
        params: [],
        action: 'list_sessions',
    },
    {
        name: 'create_session',
        description: 'Create a new Flutter debug session. After creation, connect_session and then run_app.',
        params: [
            { name: 'name', type: 'string', description: 'Human-readable name for the session', required: true },
            { name: 'projectPath', type: 'string', description: 'Absolute path to the Flutter project', required: true },
        ],
        action: 'create_session',
    },
    {
        name: 'connect_session',
        description: 'Connect to an existing session. Required before using app tools.',
        params: [
            { name: 'sessionId', type: 'string', description: 'The session ID to connect to', required: true },
        ],
        action: 'connect_session',
    },
    {
        name: 'destroy_session',
        description: 'Destroy a session and stop its app.',
        params: [
            { name: 'sessionId', type: 'string', description: 'The session ID to destroy', required: true },
        ],
        action: 'destroy_session',
    },
    // App Lifecycle
    {
        name: 'run_app',
        description: 'Start the Flutter app in the connected session.',
        params: [
            { name: 'device', type: 'string', description: 'Device ID to run on (optional)' },
            { name: 'flavor', type: 'string', description: 'Build flavor (optional)' },
            { name: 'target', type: 'string', description: 'Target file (optional)' },
        ],
        action: 'run_app',
    },
    {
        name: 'stop_app',
        description: 'Stop the running Flutter app.',
        params: [],
        action: 'stop_app',
    },
    {
        name: 'hot_reload',
        description: 'Hot reload the app. Preserves state. Use for UI-only changes.',
        params: [
            { name: 'clearRuntimeErrors', type: 'boolean', description: 'Clear runtime errors after reload' },
        ],
        action: 'hot_reload',
    },
    {
        name: 'hot_restart',
        description: 'Hot restart the app. Resets state. Use for state/logic changes.',
        params: [
            { name: 'clearRuntimeErrors', type: 'boolean', description: 'Clear runtime errors after restart' },
        ],
        action: 'hot_restart',
    },
    {
        name: 'get_status',
        description: 'Get daemon and session status. Shows if app is running, VM connected, etc.',
        params: [],
        action: 'get_status',
    },
    // Runtime Investigation
    {
        name: 'get_tree',
        description: 'Get the interaction tree - all widgets with InteractionKey.',
        params: [
            { name: 'summaryOnly', type: 'boolean', description: 'Return only widget IDs and descriptions' },
        ],
        action: 'get_tree',
    },
    {
        name: 'get_logs',
        description: 'Get recent app logs. Useful for finding errors and debug output.',
        params: [
            { name: 'maxLines', type: 'number', description: 'Maximum number of log lines (default: 100)' },
        ],
        action: 'get_logs',
    },
    {
        name: 'get_errors',
        description: 'Get runtime Flutter errors with stack traces.',
        params: [],
        action: 'get_errors',
    },
    {
        name: 'get_state',
        description: 'Get the current state of a widget by its InteractionKey ID.',
        params: [
            { name: 'nodeId', type: 'string', description: 'The InteractionKey ID of the widget', required: true },
        ],
        action: 'get_state',
    },
    // Interactions
    {
        name: 'execute',
        description: `Execute an interaction on a widget. Interactions: tap, doubleTap, longPress, enterText (requires text arg), clearText, scroll (requires dx/dy), scrollIntoView, waitFor (requires timeout), executeAction (requires action).`,
        params: [
            { name: 'nodeId', type: 'string', description: 'The InteractionKey ID', required: true },
            { name: 'interaction', type: 'string', description: 'The interaction type', required: true },
            { name: 'text', type: 'string', description: 'Text for enterText interaction' },
            { name: 'dx', type: 'number', description: 'Horizontal scroll delta' },
            { name: 'dy', type: 'number', description: 'Vertical scroll delta' },
            { name: 'timeout', type: 'number', description: 'Timeout in ms for waitFor' },
            { name: 'action', type: 'string', description: 'Semantic action for executeAction' },
        ],
        action: 'execute_interaction',
    },
    {
        name: 'batch',
        description: 'Execute multiple interactions in sequence.',
        params: [
            { name: 'steps', type: 'array', description: 'Array of {nodeId, interaction, args?, delayMs?}', required: true },
        ],
        action: 'batch',
    },
];
/**
 * Generate the toolbox script content for a tool.
 */
function generateScript(tool, daemonUri) {
    const paramDescriptions = tool.params.map(p => `${p.name}: ${p.type} ${p.description}${p.required ? ' (required)' : ''}`).join('\n');
    return `#!/usr/bin/env node
/**
 * Toolbox script: ${tool.name}
 * Auto-generated by fleeter-daemon
 */

const WebSocket = require('ws');

const DAEMON_URI = '${daemonUri}';
const ACTION = '${tool.action}';

async function describe() {
  const lines = [
    'name: ${tool.name}',
    'description: ${tool.description.replace(/'/g, "\\'")}',
${tool.params.map(p => `    '${p.name}: ${p.type} ${p.description.replace(/'/g, "\\'")}',`).join('\n')}
  ];
  process.stdout.write(lines.filter(Boolean).join('\\n'));
}

async function execute() {
  // Read params from stdin
  const input = require('fs').readFileSync(0, 'utf-8');
  const params = {};
  
  // Parse key: value format from stdin
  for (const line of input.split('\\n')) {
    const colonIdx = line.indexOf(':');
    if (colonIdx > 0) {
      const key = line.slice(0, colonIdx).trim();
      let value = line.slice(colonIdx + 1).trim();
      
      // Parse value types
      if (value === 'true') value = true;
      else if (value === 'false') value = false;
      else if (!isNaN(Number(value)) && value !== '') value = Number(value);
      else if (value.startsWith('[') || value.startsWith('{')) {
        try { value = JSON.parse(value); } catch {}
      }
      
      params[key] = value;
    }
  }

  return new Promise((resolve, reject) => {
    const ws = new WebSocket(DAEMON_URI);
    const requestId = 'toolbox-' + Date.now() + '-' + Math.random().toString(36).slice(2);
    let resolved = false;
    
    const timeout = setTimeout(() => {
      if (!resolved) {
        resolved = true;
        ws.close();
        reject(new Error('Timeout waiting for daemon response'));
      }
    }, 30000);

    ws.on('open', () => {
      // Send hello
      ws.send(JSON.stringify({
        type: 'hello',
        clientType: 'toolbox',
        clientName: '${tool.name}',
      }));
    });

    ws.on('message', (data) => {
      try {
        const msg = JSON.parse(data.toString());
        
        // After hello_ack, send command
        if (msg.type === 'hello_ack') {
          // Build the command data
          let cmdData = { ...params };
          
          // Special handling for execute_interaction
          if (ACTION === 'execute_interaction') {
            const args = {};
            if (params.text !== undefined) args.text = params.text;
            if (params.dx !== undefined) args.dx = params.dx;
            if (params.dy !== undefined) args.dy = params.dy;
            if (params.timeout !== undefined) args.timeout = params.timeout;
            if (params.action !== undefined) args.action = params.action;
            
            cmdData = {
              nodeId: params.nodeId,
              interaction: params.interaction,
              args: Object.keys(args).length > 0 ? args : undefined,
            };
          }
          
          ws.send(JSON.stringify({
            type: 'command',
            id: requestId,
            action: ACTION,
            data: cmdData,
          }));
        }
        
        // Handle response
        if (msg.type === 'response' && msg.id === requestId) {
          clearTimeout(timeout);
          resolved = true;
          ws.close();
          
          if (msg.success) {
            process.stdout.write(JSON.stringify(msg.data ?? { success: true }, null, 2));
            resolve();
          } else {
            process.stdout.write(JSON.stringify({ error: msg.error }, null, 2));
            resolve(); // Don't reject, just output error
          }
        }
      } catch (err) {
        // Ignore parse errors
      }
    });

    ws.on('error', (err) => {
      if (!resolved) {
        clearTimeout(timeout);
        resolved = true;
        process.stdout.write(JSON.stringify({ error: err.message }, null, 2));
        resolve();
      }
    });

    ws.on('close', () => {
      if (!resolved) {
        clearTimeout(timeout);
        resolved = true;
        process.stdout.write(JSON.stringify({ error: 'Connection closed' }, null, 2));
        resolve();
      }
    });
  });
}

const action = process.env.TOOLBOX_ACTION;
if (action === 'describe') {
  describe();
} else if (action === 'execute') {
  execute().catch(err => {
    process.stdout.write(JSON.stringify({ error: err.message }, null, 2));
    process.exit(1);
  });
} else {
  console.error('Unknown TOOLBOX_ACTION:', action);
  process.exit(1);
}
`;
}
/**
 * Generate a toolbox directory with all tool scripts.
 * Returns the path to the generated toolbox.
 */
export function generateToolbox(daemonUri = 'ws://127.0.0.1:9877') {
    // Create unique temp directory
    const toolboxPath = join(tmpdir(), `fleeter-toolbox-${Date.now()}`);
    mkdirSync(toolboxPath, { recursive: true });
    // Generate script for each tool
    for (const tool of TOOLS) {
        const scriptPath = join(toolboxPath, tool.name);
        const content = generateScript(tool, daemonUri);
        writeFileSync(scriptPath, content, 'utf-8');
        chmodSync(scriptPath, 0o755); // Make executable
    }
    return toolboxPath;
}
/**
 * Clean up a generated toolbox directory.
 */
export function cleanupToolbox(toolboxPath) {
    try {
        rmSync(toolboxPath, { recursive: true, force: true });
    }
    catch {
        // Ignore cleanup errors
    }
}
/**
 * Get the list of tool names for reference.
 */
export function getToolNames() {
    return TOOLS.map(t => t.name);
}
//# sourceMappingURL=toolbox-generator.js.map