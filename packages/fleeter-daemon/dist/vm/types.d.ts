/**
 * Types for the interaction tree and dart tooling.
 */
export interface InteractionTarget {
    id: string;
    description?: string;
    capabilities: InteractionCapability[];
    actions?: InteractionAction[];
    children?: InteractionTarget[];
    bounds?: TargetBounds;
    widgetType?: string;
    state?: TargetState;
}
export interface InteractionCapability {
    type: 'tap' | 'doubleTap' | 'longPress' | 'enterText' | 'scroll' | 'drag';
}
export interface InteractionAction {
    name: string;
    description?: string;
    parameters?: ActionParameter[];
}
export interface ActionParameter {
    name: string;
    type: 'string' | 'int' | 'double' | 'bool';
    description?: string;
    required?: boolean;
    defaultValue?: unknown;
}
export interface TargetBounds {
    x: number;
    y: number;
    width: number;
    height: number;
}
export interface TargetState {
    text?: string;
    enabled?: boolean;
    visible?: boolean;
    focused?: boolean;
}
export interface InteractionResult {
    success: boolean;
    error?: string;
    duration_ms?: number;
    tree?: InteractionTarget[];
}
export interface BatchStep {
    action: 'tap' | 'doubleTap' | 'longPress' | 'enterText' | 'clearText' | 'drag' | 'scroll' | 'scrollIntoView' | 'waitFor' | 'executeAction';
    id: string;
    text?: string;
    dx?: number;
    dy?: number;
    alignment?: number;
    actionName?: string;
    args?: Record<string, unknown>;
    condition?: 'exists' | 'notExists' | 'visible' | 'notVisible';
    timeoutMs?: number;
    settle?: boolean;
}
export interface BatchResult {
    success: boolean;
    results: InteractionResult[];
    stoppedAtIndex?: number;
    error?: string;
    tree?: InteractionTarget[];
}
export interface GetTreeOptions {
    includeBounds?: boolean;
    includeWidgetType?: boolean;
    includeState?: boolean;
    summaryOnly?: boolean;
}
export interface LogEntry {
    timestamp: string;
    level: 'info' | 'warning' | 'error' | 'debug';
    message: string;
    logger?: string;
}
export interface RuntimeError {
    message: string;
    stackTrace?: string;
    timestamp?: string;
}
export interface HotReloadResult {
    success: boolean;
    reloadedAt?: string;
    restartedAt?: string;
    error?: string;
}
export interface AppStatus {
    connected: boolean;
    isolateId?: string;
    appUri?: string;
    paused?: boolean;
}
//# sourceMappingURL=types.d.ts.map