// apps/studio/src/performance/index.ts — CR-V2-B6-021.
export const INITIAL_LOAD_BUDGET_MS = 1500;
export const INTERACTION_BUDGET_MS = 100;
export const MEMORY_BUDGET_MB = 350;
export function measure<T>(label: string, fn: () => T): T { return fn(); }
