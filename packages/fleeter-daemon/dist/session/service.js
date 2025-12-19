/**
 * SessionService - unified service for all session operations.
 * One instance per active session, wrapping VMServiceClient, SessionManager, and FlutterProcessManager.
 */
import { EventEmitter } from 'events';
export class SessionService extends EventEmitter {
    sessionId;
    vmClient;
    sessionManager;
    processManager;
    projectPath;
    treeCache = null;
    treeCacheTime = 0;
    treeCacheTtlMs = 500;
    constructor(options) {
        super();
        this.sessionId = options.sessionId;
        this.vmClient = options.vmClient;
        this.sessionManager = options.sessionManager;
        this.processManager = options.processManager;
        this.projectPath = options.projectPath;
    }
    /** Set the VM client (called when VM connects after runApp) */
    setVmClient(vmClient) {
        this.vmClient = vmClient;
    }
    /** Clear the VM client (called when app exits but session persists) */
    clearVmClient() {
        this.vmClient = undefined;
        this.invalidateTreeCache();
    }
    get id() {
        return this.sessionId;
    }
    get isVmConnected() {
        return this.vmClient?.isConnected ?? false;
    }
    get client() {
        return this.vmClient;
    }
    // ─────────────────────────────────────────────────────────────────────────────
    // VMServiceClient methods (require VM connection)
    // ─────────────────────────────────────────────────────────────────────────────
    async getTree(options) {
        const client = this.requireVmConnection();
        const now = Date.now();
        if (this.treeCache && now - this.treeCacheTime < this.treeCacheTtlMs) {
            return this.treeCache;
        }
        const tree = await client.getTree(options);
        this.treeCache = tree;
        this.treeCacheTime = now;
        return tree;
    }
    async execute(nodeId, interaction, args) {
        const client = this.requireVmConnection();
        const result = await client.execute(nodeId, interaction, args);
        // Update cache from the returned tree (source of truth after settle)
        if (result.tree) {
            this.treeCache = result.tree;
            this.treeCacheTime = Date.now();
        }
        // Emit interaction event for monitoring
        this.emit('interaction', {
            sessionId: this.sessionId,
            nodeId,
            interaction,
            args,
            success: result.success,
            error: result.error,
        });
        return result;
    }
    async getState(nodeId) {
        const client = this.requireVmConnection();
        return client.getState(nodeId);
    }
    async batch(steps) {
        const client = this.requireVmConnection();
        const result = await client.batch(steps);
        // Update cache from the returned tree
        if (result.tree) {
            this.treeCache = result.tree;
            this.treeCacheTime = Date.now();
        }
        return result;
    }
    async hotReload(clearErrors = false) {
        const client = this.requireVmConnection();
        // Invalidate cache - hot reload changes the tree but doesn't return it
        this.invalidateTreeCache();
        return client.hotReload(clearErrors);
    }
    async hotRestart(clearErrors = true) {
        const client = this.requireVmConnection();
        // Invalidate cache - hot restart changes the tree but doesn't return it
        this.invalidateTreeCache();
        return client.hotRestart(clearErrors);
    }
    async getRuntimeErrors() {
        const client = this.requireVmConnection();
        return client.getRuntimeErrors();
    }
    clearRuntimeErrors() {
        if (this.vmClient?.isConnected) {
            this.vmClient.clearRuntimeErrors();
        }
    }
    // ─────────────────────────────────────────────────────────────────────────────
    // SessionManager methods
    // ─────────────────────────────────────────────────────────────────────────────
    getLogs(maxLines = 100) {
        return this.sessionManager.getLogs(this.sessionId, maxLines);
    }
    getStatus() {
        const session = this.sessionManager.get(this.sessionId);
        if (!session) {
            throw new Error(`Session not found: ${this.sessionId}`);
        }
        return {
            sessionInfo: this.sessionManager.toInfo(session),
            vmConnected: this.vmClient?.isConnected ?? false,
            vmServiceUri: this.vmClient?.connectionUri ?? undefined,
        };
    }
    // ─────────────────────────────────────────────────────────────────────────────
    // FlutterProcessManager methods
    // ─────────────────────────────────────────────────────────────────────────────
    async runApp(options) {
        await this.processManager.runApp(this.sessionId, this.projectPath, options);
    }
    async stopApp() {
        await this.processManager.stopApp(this.sessionId);
        this.invalidateTreeCache();
    }
    // ─────────────────────────────────────────────────────────────────────────────
    // Internal helpers
    // ─────────────────────────────────────────────────────────────────────────────
    requireVmConnection() {
        if (!this.vmClient || !this.vmClient.isConnected) {
            throw new Error('VM service not connected. Start the app with runApp() first, or wait for the VM to connect.');
        }
        return this.vmClient;
    }
    invalidateTreeCache() {
        this.treeCache = null;
        this.treeCacheTime = 0;
    }
}
//# sourceMappingURL=service.js.map