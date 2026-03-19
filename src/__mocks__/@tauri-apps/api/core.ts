import { vi } from 'vitest'

// Track all invoke calls for assertions
export const invokeCalls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];

// Configurable mock results per command
const mockResults: Record<string, unknown> = {};

// The mock invoke function
export const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
  // Record the call
  invokeCalls.push({ cmd, args });

  // CONTRACT CHECK: No undefined values in args
  // Tauri 2 rejects these with "missing required key X"
  if (args) {
    for (const [key, value] of Object.entries(args)) {
      if (value === undefined) {
        throw new Error(
          `CONTRACT VIOLATION: invoke("${cmd}") param "${key}" is undefined. ` +
          `Tauri will reject with "command ${cmd} missing required key ${key}".`
        );
      }
    }
  }

  // Return configured mock result
  if (cmd in mockResults) {
    const result = mockResults[cmd];
    return typeof result === 'function' ? (result as (a?: Record<string, unknown>) => unknown)(args) : result;
  }

  return null;
});

// Helper: configure mock result for a command
export function mockInvokeResult(cmd: string, result: unknown) {
  mockResults[cmd] = result;
}

// Helper: clear all recorded calls and mock results
export function clearInvokeMocks() {
  invokeCalls.length = 0;
  Object.keys(mockResults).forEach(k => delete mockResults[k]);
  invoke.mockClear();
}

// Helper: get calls for a specific command
export function getInvokeCalls(cmd: string) {
  return invokeCalls.filter(c => c.cmd === cmd);
}

// Helper: assert a command was called with specific args
export function assertInvoked(cmd: string, expectedArgs?: Record<string, unknown>) {
  const calls = getInvokeCalls(cmd);
  if (calls.length === 0) {
    throw new Error(`Expected invoke("${cmd}") to be called, but it was not`);
  }
  if (expectedArgs) {
    const lastCall = calls[calls.length - 1];
    for (const [key, value] of Object.entries(expectedArgs)) {
      if (lastCall.args?.[key] !== value) {
        throw new Error(
          `Expected invoke("${cmd}") param "${key}" to be ${JSON.stringify(value)}, ` +
          `got ${JSON.stringify(lastCall.args?.[key])}`
        );
      }
    }
  }
}

export function convertFileSrc(path: string): string {
  return path;
}
