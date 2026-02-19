/**
 * Structured logging for Fleeter daemon.
 * By default, logs only info/warn/error. Use --verbose for debug logs.
 */
import pino from 'pino';
import pinoPretty from 'pino-pretty';
import fs from 'fs';
import path from 'path';
import os from 'os';
const FLEETER_DIR = path.join(os.homedir(), '.fleeter');
const LOGS_DIR = path.join(FLEETER_DIR, 'logs');
let verbose = false;
let _logger = null;
export function setVerbose(v) {
    verbose = v;
}
export function isVerbose() {
    return verbose;
}
function ensureLogDir() {
    if (!fs.existsSync(LOGS_DIR)) {
        fs.mkdirSync(LOGS_DIR, { recursive: true });
    }
}
function getLogFilePath() {
    const date = new Date().toISOString().split('T')[0];
    return path.join(LOGS_DIR, `daemon-${date}.log`);
}
function getLogger() {
    if (_logger)
        return _logger;
    ensureLogDir();
    const consoleLevel = verbose ? 'debug' : 'info';
    _logger = pino({
        name: 'fleeter',
        level: 'trace',
        base: undefined,
    }, pino.multistream([
        {
            level: 'trace',
            stream: pino.destination({
                dest: getLogFilePath(),
                sync: true,
            }),
        },
        {
            level: consoleLevel,
            stream: pinoPretty({
                destination: 2,
                colorize: true,
                translateTime: 'HH:MM:ss',
                ignore: 'pid,hostname',
            }),
        },
    ]));
    return _logger;
}
export const createLogger = (module) => {
    return new Proxy({}, {
        get(_, prop) {
            const logger = getLogger().child({ module });
            return logger[prop];
        },
    });
};
export const log = {
    daemon: createLogger('daemon'),
    vm: createLogger('vm'),
    flutter: createLogger('flutter'),
    session: createLogger('session'),
    ws: createLogger('ws'),
    tree: createLogger('tree'),
    agent: createLogger('agent'),
};
//# sourceMappingURL=logger.js.map