/**
 * Structured logging for Fleeter daemon.
 * By default, logs only info/warn/error. Use --verbose for debug logs.
 */
import pino, { type Logger as PinoLogger } from 'pino';
export declare function setVerbose(v: boolean): void;
export declare function isVerbose(): boolean;
export declare const createLogger: (module: string) => pino.Logger;
export declare const log: {
    daemon: pino.Logger;
    vm: pino.Logger;
    flutter: pino.Logger;
    session: pino.Logger;
    ws: pino.Logger;
    tree: pino.Logger;
    agent: pino.Logger;
};
export type Logger = PinoLogger;
//# sourceMappingURL=logger.d.ts.map