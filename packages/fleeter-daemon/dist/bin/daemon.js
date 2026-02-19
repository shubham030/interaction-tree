#!/usr/bin/env node
/**
 * Fleeter Daemon CLI
 */
import { Command } from 'commander';
import { Daemon } from '../daemon.js';
import { setVerbose } from '../logger.js';
const program = new Command();
program
    .name('fleeter-daemon')
    .description('Central daemon for Flutter session management')
    .version('0.1.0')
    .option('-p, --port <port>', 'Port to listen on', '9877')
    .option('--host <host>', 'Host to bind to', '127.0.0.1')
    .option('-v, --verbose', 'Enable verbose logging')
    .action(async (options) => {
    if (options.verbose) {
        setVerbose(true);
    }
    const port = parseInt(options.port, 10);
    const host = options.host;
    const daemon = new Daemon({ port, host });
    await daemon.start();
    // Keep process running
    process.stdin.resume();
});
program.parse();
//# sourceMappingURL=daemon.js.map