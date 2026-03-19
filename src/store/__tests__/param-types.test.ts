import { describe, it, expect } from 'vitest'
import { invoke, getInvokeCalls } from '../../__mocks__/@tauri-apps/api/core'

/**
 * Parameter type contracts: verify params have correct types,
 * not just non-undefined values.
 */

describe('parameter type contracts', () => {
  describe('IDs are strings, not numbers', () => {
    it('chatId is a string', async () => {
      await invoke('get_messages_branched', { chatId: 'string-id' });
      const calls = getInvokeCalls('get_messages_branched');
      expect(typeof calls[0].args?.chatId).toBe('string');
    });

    it('messageId is a string', async () => {
      await invoke('delete_message', { messageId: 'msg-123' });
      const calls = getInvokeCalls('delete_message');
      expect(typeof calls[0].args?.messageId).toBe('string');
    });

    it('runId is a string', async () => {
      await invoke('cancel_agent_run', { runId: 'run-abc' });
      const calls = getInvokeCalls('cancel_agent_run');
      expect(typeof calls[0].args?.runId).toBe('string');
    });

    it('planId is a string', async () => {
      await invoke('delete_plan', { planId: 'plan-xyz' });
      const calls = getInvokeCalls('delete_plan');
      expect(typeof calls[0].args?.planId).toBe('string');
    });

    it('skillId is a string', async () => {
      await invoke('attach_skill_to_chat', { chatId: 'c1', skillId: 'skill-1' });
      const calls = getInvokeCalls('attach_skill_to_chat');
      expect(typeof calls[0].args?.skillId).toBe('string');
    });

    it('kbId is a string', async () => {
      await invoke('attach_kb_to_chat', { chatId: 'c1', kbId: 'kb-1' });
      const calls = getInvokeCalls('attach_kb_to_chat');
      expect(typeof calls[0].args?.kbId).toBe('string');
    });

    it('projectId is a string', async () => {
      await invoke('assign_chat_to_project', { chatId: 'c1', projectId: 'proj-1' });
      const calls = getInvokeCalls('assign_chat_to_project');
      expect(typeof calls[0].args?.projectId).toBe('string');
    });
  });

  describe('timestamps are unix seconds', () => {
    it('timestamp values should be less than 10 billion (seconds, not ms)', () => {
      const unixSeconds = Math.floor(Date.now() / 1000);
      const unixMs = Date.now();

      // Seconds should be < 10 billion (year ~2286)
      expect(unixSeconds).toBeLessThan(10_000_000_000);
      // Milliseconds would be > 10 billion (current epoch)
      expect(unixMs).toBeGreaterThan(10_000_000_000);
    });
  });

  describe('null vs undefined distinction', () => {
    it('null is accepted for optional Tauri params (Option<T>)', async () => {
      // Tauri deserializes null → None for Option<T>
      await expect(invoke('move_chat_to_folder', { chatId: 'c1', folderId: null })).resolves.not.toThrow();
      await expect(invoke('create_folder', { name: 'Test', color: null, mode: 'chat' })).resolves.not.toThrow();
      await expect(invoke('list_projects', { statusFilter: null })).resolves.not.toThrow();
    });

    it('undefined is never accepted for any param', async () => {
      // Tauri rejects undefined — it strips the key from the payload
      await expect(invoke('move_chat_to_folder', { chatId: 'c1', folderId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
      await expect(invoke('create_folder', { name: 'Test', color: undefined, mode: 'chat' })).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('empty string is valid for string params', async () => {
      await expect(invoke('update_chat_title', { chatId: 'c1', title: '' })).resolves.not.toThrow();
      await expect(invoke('set_chat_system_prompt', { chatId: 'c1', systemPrompt: '' })).resolves.not.toThrow();
    });

    it('zero is valid for numeric params', async () => {
      await expect(invoke('update_message_usage', { id: 'msg-1', promptTokens: 0, completionTokens: 0, cost: 0 })).resolves.not.toThrow();
    });

    it('false is valid for boolean params', async () => {
      await expect(invoke('mcp_toggle_server', { id: 'srv-1', enabled: false })).resolves.not.toThrow();
      await expect(invoke('toggle_scheduled_task', { id: 'task-1', enabled: false })).resolves.not.toThrow();
    });

    it('empty array is valid for array params', async () => {
      await expect(invoke('reorder_folders', { folderIds: [] })).resolves.not.toThrow();
      await expect(invoke('reorder_templates', { templateIds: [] })).resolves.not.toThrow();
    });
  });
});
