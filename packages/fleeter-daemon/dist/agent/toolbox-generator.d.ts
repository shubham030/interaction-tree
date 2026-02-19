/**
 * Toolbox Generator - Creates executable scripts for the Debug Agent.
 *
 * These scripts communicate with the daemon via WebSocket to execute
 * commands like list_sessions, create_session, get_tree, etc.
 *
 * The debug agent uses these via the Amp SDK's `toolbox` option.
 */
/**
 * Generate a toolbox directory with all tool scripts.
 * Returns the path to the generated toolbox.
 */
export declare function generateToolbox(daemonUri?: string): string;
/**
 * Clean up a generated toolbox directory.
 */
export declare function cleanupToolbox(toolboxPath: string): void;
/**
 * Get the list of tool names for reference.
 */
export declare function getToolNames(): string[];
//# sourceMappingURL=toolbox-generator.d.ts.map