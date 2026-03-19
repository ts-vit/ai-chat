import { describe, it, expect } from 'vitest'
import { invoke, getInvokeCalls } from '../../__mocks__/@tauri-apps/api/core'

/**
 * Contract tests: verify invoke() calls reject undefined params.
 * Tauri 2 crashes with "missing required key X" when undefined is passed.
 * These tests call invoke() directly — no Zustand store needed.
 */

describe('Tauri invoke contracts', () => {

  // --- Message commands ---

  describe('message commands', () => {
    it('update_message_content rejects undefined content', async () => {
      await expect(
        invoke('update_message_content', { id: 'msg-1', content: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_message_content rejects undefined id', async () => {
      await expect(
        invoke('update_message_content', { id: undefined, content: 'text' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_message_content accepts null content', async () => {
      await expect(
        invoke('update_message_content', { id: 'msg-1', content: null })
      ).resolves.not.toThrow();
    });

    it('update_message_content accepts empty string', async () => {
      await expect(
        invoke('update_message_content', { id: 'msg-1', content: '' })
      ).resolves.not.toThrow();
    });

    it('send_message rejects undefined chatId', async () => {
      await expect(
        invoke('send_message', { chatId: undefined, content: 'hello', model: 'gpt-4', baseUrl: 'url', apiKey: 'key', systemPrompt: '', messageId: 'm1', tempId: 't1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('send_message rejects undefined model', async () => {
      await expect(
        invoke('send_message', { chatId: 'c1', content: 'hello', model: undefined, baseUrl: 'url', apiKey: 'key', systemPrompt: '', messageId: 'm1', tempId: 't1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('send_message rejects undefined content', async () => {
      await expect(
        invoke('send_message', { chatId: 'c1', content: undefined, model: 'gpt-4', baseUrl: 'url', apiKey: 'key', systemPrompt: '', messageId: 'm1', tempId: 't1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_message rejects undefined messageId', async () => {
      await expect(
        invoke('delete_message', { messageId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_message_usage rejects undefined id', async () => {
      await expect(
        invoke('update_message_usage', { id: undefined, promptTokens: 10, completionTokens: 20, cost: 0.01 })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_message_usage rejects undefined cost', async () => {
      await expect(
        invoke('update_message_usage', { id: 'msg-1', promptTokens: 10, completionTokens: 20, cost: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('index_message rejects undefined messageId', async () => {
      await expect(
        invoke('index_message', { messageId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_message_web_sources rejects undefined messageId', async () => {
      await expect(
        invoke('update_message_web_sources', { messageId: undefined, webSources: '[]' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Chat commands ---

  describe('chat commands', () => {
    it('create_chat rejects undefined mode', async () => {
      await expect(
        invoke('create_chat', { mode: undefined, model: 'gpt-4', title: '', systemPrompt: '', providerId: 'openrouter' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('create_chat rejects undefined model', async () => {
      await expect(
        invoke('create_chat', { mode: 'chat', model: undefined, title: '', systemPrompt: '', providerId: 'openrouter' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('create_chat accepts valid params', async () => {
      await invoke('create_chat', { mode: 'chat', model: 'gpt-4', title: '', systemPrompt: '', providerId: 'openrouter', isImageModel: false });
      const calls = getInvokeCalls('create_chat');
      expect(calls).toHaveLength(1);
      expect(calls[0].args?.mode).toBe('chat');
      expect(calls[0].args?.model).toBe('gpt-4');
    });

    it('update_chat_title rejects undefined chatId', async () => {
      await expect(
        invoke('update_chat_title', { chatId: undefined, title: 'Test' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_chat_title rejects undefined title', async () => {
      await expect(
        invoke('update_chat_title', { chatId: 'c1', title: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_chat rejects undefined id', async () => {
      await expect(
        invoke('delete_chat', { id: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('get_messages_branched rejects undefined chatId', async () => {
      await expect(
        invoke('get_messages_branched', { chatId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_chat_model rejects undefined chatId', async () => {
      await expect(
        invoke('update_chat_model', { chatId: undefined, model: 'gpt-4', isImageModel: false })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_chat_model rejects undefined model', async () => {
      await expect(
        invoke('update_chat_model', { chatId: 'c1', model: undefined, isImageModel: false })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('set_chat_system_prompt rejects undefined chatId', async () => {
      await expect(
        invoke('set_chat_system_prompt', { chatId: undefined, systemPrompt: 'test' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('move_chat_to_folder accepts null folderId', async () => {
      await expect(
        invoke('move_chat_to_folder', { chatId: 'c1', folderId: null })
      ).resolves.not.toThrow();
    });
  });

  // --- Agent commands ---

  describe('agent commands', () => {
    it('send_agent_message rejects undefined chatId', async () => {
      await expect(
        invoke('send_agent_message', { chatId: undefined, content: 'test', model: 'gpt-4', baseUrl: 'url', apiKey: 'key', systemPrompt: '', userMessageId: 'u1', assistantMessageId: 'a1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('send_agent_message rejects undefined content', async () => {
      await expect(
        invoke('send_agent_message', { chatId: 'c1', content: undefined, model: 'gpt-4', baseUrl: 'url', apiKey: 'key', systemPrompt: '', userMessageId: 'u1', assistantMessageId: 'a1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('send_agent_message rejects undefined model', async () => {
      await expect(
        invoke('send_agent_message', { chatId: 'c1', content: 'test', model: undefined, baseUrl: 'url', apiKey: 'key', systemPrompt: '', userMessageId: 'u1', assistantMessageId: 'a1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('cancel_agent_run rejects undefined runId', async () => {
      await expect(
        invoke('cancel_agent_run', { runId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('resume_agent_run rejects undefined runId', async () => {
      await expect(
        invoke('resume_agent_run', { runId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('get_agent_runs rejects undefined chatId', async () => {
      await expect(
        invoke('get_agent_runs', { chatId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Memory commands ---

  describe('memory commands', () => {
    it('create_agent_memory rejects undefined content', async () => {
      await expect(
        invoke('create_agent_memory', { content: undefined, category: 'fact' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('create_agent_memory rejects undefined category', async () => {
      await expect(
        invoke('create_agent_memory', { content: 'test', category: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_agent_memory rejects undefined id', async () => {
      await expect(
        invoke('update_agent_memory', { id: undefined, content: 'updated' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_agent_memory rejects undefined id', async () => {
      await expect(
        invoke('delete_agent_memory', { id: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Skills commands ---

  describe('skill commands', () => {
    it('attach_skill_to_chat rejects undefined chatId', async () => {
      await expect(
        invoke('attach_skill_to_chat', { chatId: undefined, skillId: 'skill-1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('attach_skill_to_chat rejects undefined skillId', async () => {
      await expect(
        invoke('attach_skill_to_chat', { chatId: 'c1', skillId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('detach_skill_from_chat rejects undefined chatId', async () => {
      await expect(
        invoke('detach_skill_from_chat', { chatId: undefined, skillId: 'skill-1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('detach_skill_from_chat rejects undefined skillId', async () => {
      await expect(
        invoke('detach_skill_from_chat', { chatId: 'c1', skillId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('get_chat_skills rejects undefined chatId', async () => {
      await expect(
        invoke('get_chat_skills', { chatId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Knowledge Base commands ---

  describe('KB commands', () => {
    it('attach_kb_to_chat rejects undefined chatId', async () => {
      await expect(
        invoke('attach_kb_to_chat', { chatId: undefined, kbId: 'kb-1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('attach_kb_to_chat rejects undefined kbId', async () => {
      await expect(
        invoke('attach_kb_to_chat', { chatId: 'c1', kbId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('search_knowledge_base rejects undefined kbId', async () => {
      await expect(
        invoke('search_knowledge_base', { kbId: undefined, query: 'test' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('search_knowledge_base rejects undefined query', async () => {
      await expect(
        invoke('search_knowledge_base', { kbId: 'kb-1', query: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('detach_kb_from_chat rejects undefined chatId', async () => {
      await expect(
        invoke('detach_kb_from_chat', { chatId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Workspace commands ---

  describe('workspace commands', () => {
    it('create_workspace_artifact rejects undefined name', async () => {
      await expect(
        invoke('create_workspace_artifact', { chatId: 'c1', name: undefined, content: 'test', contentType: 'text' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('create_workspace_artifact rejects undefined content', async () => {
      await expect(
        invoke('create_workspace_artifact', { chatId: 'c1', name: 'file.txt', content: undefined, contentType: 'text' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_workspace_artifact rejects undefined id', async () => {
      await expect(
        invoke('delete_workspace_artifact', { id: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_workspace_artifact rejects undefined id', async () => {
      await expect(
        invoke('update_workspace_artifact', { id: undefined, name: 'file.txt', content: 'test' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Project commands ---

  describe('project commands', () => {
    it('create_project rejects undefined name', async () => {
      await expect(
        invoke('create_project', { name: undefined, goal: 'test' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('assign_chat_to_project rejects undefined chatId', async () => {
      await expect(
        invoke('assign_chat_to_project', { chatId: undefined, projectId: 'proj-1' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('assign_chat_to_project rejects undefined projectId', async () => {
      await expect(
        invoke('assign_chat_to_project', { chatId: 'c1', projectId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_project rejects undefined id', async () => {
      await expect(
        invoke('delete_project', { id: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('remove_chat_from_project rejects undefined chatId', async () => {
      await expect(
        invoke('remove_chat_from_project', { chatId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Plan commands ---

  describe('plan commands', () => {
    it('start_plan_execution rejects undefined planId', async () => {
      await expect(
        invoke('start_plan_execution', { planId: undefined, model: 'gpt-4', baseUrl: 'url', apiKey: 'key', systemPrompt: '' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('start_plan_execution rejects undefined model', async () => {
      await expect(
        invoke('start_plan_execution', { planId: 'p1', model: undefined, baseUrl: 'url', apiKey: 'key', systemPrompt: '' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('delete_plan rejects undefined planId', async () => {
      await expect(
        invoke('delete_plan', { planId: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });

  // --- Folder commands ---

  describe('folder commands', () => {
    it('create_folder rejects undefined name', async () => {
      await expect(
        invoke('create_folder', { name: undefined, color: null, mode: 'chat' })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('create_folder accepts null color', async () => {
      await expect(
        invoke('create_folder', { name: 'Test', color: null, mode: 'chat' })
      ).resolves.not.toThrow();
    });

    it('delete_folder rejects undefined id', async () => {
      await expect(
        invoke('delete_folder', { id: undefined })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });

    it('update_folder rejects undefined id', async () => {
      await expect(
        invoke('update_folder', { id: undefined, name: 'Test', color: null })
      ).rejects.toThrow('CONTRACT VIOLATION');
    });
  });
});
