import { describe, it, expect } from 'vitest'
import { invoke } from '../../__mocks__/@tauri-apps/api/core'

/**
 * Invoke audit: covers remaining commands not in contracts.test.ts.
 * Grouped by store slice. Tests that required params reject undefined.
 */

describe('invoke audit — settingsSlice', () => {
  it('save_settings rejects undefined settings', async () => {
    await expect(invoke('save_settings', { settings: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('save_settings accepts valid settings object', async () => {
    await expect(invoke('save_settings', { settings: '{}' })).resolves.not.toThrow();
  });

  it('create_custom_provider rejects undefined input', async () => {
    await expect(invoke('create_custom_provider', { input: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_custom_provider rejects undefined id', async () => {
    await expect(invoke('update_custom_provider', { id: undefined, input: { name: 'x', baseUrl: 'u', apiKey: 'k' } })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_custom_provider rejects undefined id', async () => {
    await expect(invoke('delete_custom_provider', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_ollama_models rejects undefined baseUrl', async () => {
    await expect(invoke('get_ollama_models', { baseUrl: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_ollama_model rejects undefined modelName', async () => {
    await expect(invoke('delete_ollama_model', { modelName: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('pull_ollama_model rejects undefined modelName', async () => {
    await expect(invoke('pull_ollama_model', { modelName: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_credits rejects undefined managementKey', async () => {
    await expect(invoke('get_credits', { managementKey: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // MCP commands
  it('mcp_add_server rejects undefined name', async () => {
    await expect(invoke('mcp_add_server', { name: undefined, command: 'cmd', args: [], env: {}, enabled: true })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('mcp_add_server rejects undefined command', async () => {
    await expect(invoke('mcp_add_server', { name: 'test', command: undefined, args: [], env: {}, enabled: true })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('mcp_update_server rejects undefined id', async () => {
    await expect(invoke('mcp_update_server', { id: undefined, name: 'test', command: 'cmd', args: [], env: {}, enabled: true })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('mcp_remove_server rejects undefined id', async () => {
    await expect(invoke('mcp_remove_server', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('mcp_toggle_server rejects undefined id', async () => {
    await expect(invoke('mcp_toggle_server', { id: undefined, enabled: true })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('mcp_connect_by_id rejects undefined serverId', async () => {
    await expect(invoke('mcp_connect_by_id', { serverId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_fs_mcp_config rejects undefined config', async () => {
    await expect(invoke('update_fs_mcp_config', { config: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_fs_audit_log rejects undefined limit', async () => {
    await expect(invoke('get_fs_audit_log', { limit: undefined, offset: 0 })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Image styles
  it('create_image_style rejects undefined name', async () => {
    await expect(invoke('create_image_style', { name: undefined, promptSuffix: 'style' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_image_style rejects undefined id', async () => {
    await expect(invoke('update_image_style', { id: undefined, name: 'style', promptSuffix: 'suffix' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_image_style rejects undefined id', async () => {
    await expect(invoke('delete_image_style', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });
});

describe('invoke audit — uiSlice', () => {
  it('update_mode_settings rejects undefined mode', async () => {
    await expect(invoke('update_mode_settings', { mode: undefined, enabled: true, sortOrder: 0, config: '{}' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_comparison_messages rejects undefined comparisonId', async () => {
    await expect(invoke('get_comparison_messages', { comparisonId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('create_comparison rejects undefined leftModel', async () => {
    await expect(invoke('create_comparison', {
      leftProviderId: 'openrouter', leftModel: undefined,
      rightProviderId: 'openrouter', rightModel: 'gpt-4',
      leftSystemPrompt: '', rightSystemPrompt: ''
    })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('create_comparison rejects undefined rightModel', async () => {
    await expect(invoke('create_comparison', {
      leftProviderId: 'openrouter', leftModel: 'gpt-4',
      rightProviderId: 'openrouter', rightModel: undefined,
      leftSystemPrompt: '', rightSystemPrompt: ''
    })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_comparison rejects undefined comparisonId', async () => {
    await expect(invoke('delete_comparison', { comparisonId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_comparison_title rejects undefined comparisonId', async () => {
    await expect(invoke('update_comparison_title', { comparisonId: undefined, title: 'Test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('validate_telegram_token rejects undefined token', async () => {
    await expect(invoke('validate_telegram_token', { token: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('authorize_telegram_user rejects undefined userId', async () => {
    await expect(invoke('authorize_telegram_user', { userId: undefined, firstName: 'John', lastName: null, username: null })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('authorize_telegram_user accepts null lastName and username', async () => {
    await expect(invoke('authorize_telegram_user', { userId: 123, firstName: 'John', lastName: null, username: null })).resolves.not.toThrow();
  });
});

describe('invoke audit — snippetsSlice', () => {
  // Presets
  it('create_preset rejects undefined name', async () => {
    await expect(invoke('create_preset', { name: undefined, content: 'test', isDefault: false })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_preset rejects undefined id', async () => {
    await expect(invoke('update_preset', { id: undefined, name: 'test', content: 'test', isDefault: false })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_preset rejects undefined id', async () => {
    await expect(invoke('delete_preset', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Categories
  it('create_category rejects undefined name', async () => {
    await expect(invoke('create_category', { name: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_category rejects undefined id', async () => {
    await expect(invoke('update_category', { id: undefined, name: 'test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_category rejects undefined id', async () => {
    await expect(invoke('delete_category', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Snippets
  it('create_snippet rejects undefined name', async () => {
    await expect(invoke('create_snippet', { name: undefined, content: 'test', categoryId: 'cat-1' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_snippet rejects undefined id', async () => {
    await expect(invoke('update_snippet', { id: undefined, name: 'test', content: 'test', categoryId: 'cat-1' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_snippet rejects undefined id', async () => {
    await expect(invoke('delete_snippet', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Templates
  it('create_template rejects undefined name', async () => {
    await expect(invoke('create_template', { name: undefined, providerId: 'openrouter', model: 'gpt-4', systemPrompt: '' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_template rejects undefined id', async () => {
    await expect(invoke('update_template', { id: undefined, name: 'test', providerId: 'openrouter', model: 'gpt-4', systemPrompt: '' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_template rejects undefined id', async () => {
    await expect(invoke('delete_template', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('reorder_templates rejects undefined templateIds', async () => {
    await expect(invoke('reorder_templates', { templateIds: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Prompt library
  it('create_prompt_library_item rejects undefined title', async () => {
    await expect(invoke('create_prompt_library_item', { title: undefined, description: 'test', content: 'test', category: 'general', language: 'en' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_prompt_library_item rejects undefined id', async () => {
    await expect(invoke('update_prompt_library_item', { id: undefined, title: 'test', description: 'test', content: 'test', category: 'general' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_prompt_library_item rejects undefined id', async () => {
    await expect(invoke('delete_prompt_library_item', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_prompt_library accepts null category and search', async () => {
    await expect(invoke('get_prompt_library', { category: null, search: null, language: 'en' })).resolves.not.toThrow();
  });
});

describe('invoke audit — kbSlice', () => {
  it('create_knowledge_base rejects undefined name', async () => {
    await expect(invoke('create_knowledge_base', { name: undefined, description: 'test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('update_knowledge_base rejects undefined id', async () => {
    await expect(invoke('update_knowledge_base', { id: undefined, name: 'test', description: 'test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_knowledge_base rejects undefined id', async () => {
    await expect(invoke('delete_knowledge_base', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('list_kb_documents rejects undefined kbId', async () => {
    await expect(invoke('list_kb_documents', { kbId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('add_kb_documents_bulk rejects undefined kbId', async () => {
    await expect(invoke('add_kb_documents_bulk', { kbId: undefined, filePaths: ['/test.txt'] })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('remove_kb_document rejects undefined kbId', async () => {
    await expect(invoke('remove_kb_document', { kbId: undefined, documentId: 'doc-1' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('remove_kb_document rejects undefined documentId', async () => {
    await expect(invoke('remove_kb_document', { kbId: 'kb-1', documentId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('index_kb_document rejects undefined kbId', async () => {
    await expect(invoke('index_kb_document', { kbId: undefined, documentId: 'doc-1' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('index_all_kb_documents rejects undefined kbId', async () => {
    await expect(invoke('index_all_kb_documents', { kbId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('reindex_knowledge_base rejects undefined kbId', async () => {
    await expect(invoke('reindex_knowledge_base', { kbId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('get_chat_kb rejects undefined chatId', async () => {
    await expect(invoke('get_chat_kb', { chatId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('export_knowledge_base rejects undefined kbId', async () => {
    await expect(invoke('export_knowledge_base', { kbId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('import_knowledge_base rejects undefined zipPath', async () => {
    await expect(invoke('import_knowledge_base', { zipPath: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });
});

describe('invoke audit — agentSlice extras', () => {
  // Scheduled tasks
  it('update_scheduled_task rejects undefined id', async () => {
    await expect(invoke('update_scheduled_task', { id: undefined, name: 'task' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_scheduled_task rejects undefined id', async () => {
    await expect(invoke('delete_scheduled_task', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('toggle_scheduled_task rejects undefined id', async () => {
    await expect(invoke('toggle_scheduled_task', { id: undefined, enabled: true })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('toggle_scheduled_task rejects undefined enabled', async () => {
    await expect(invoke('toggle_scheduled_task', { id: 'task-1', enabled: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('run_scheduled_task_now rejects undefined id', async () => {
    await expect(invoke('run_scheduled_task_now', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Skills extras
  it('update_skill rejects undefined id', async () => {
    await expect(invoke('update_skill', { id: undefined, name: 'test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  it('delete_skill rejects undefined id', async () => {
    await expect(invoke('delete_skill', { id: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Workspace extras
  it('update_workspace_artifact rejects undefined name', async () => {
    await expect(invoke('update_workspace_artifact', { id: 'a1', name: undefined, content: 'test' })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // Search
  it('delete_search_index rejects undefined chatId', async () => {
    await expect(invoke('delete_search_index', { chatId: undefined })).rejects.toThrow('CONTRACT VIOLATION');
  });

  // list_agent_memories accepts null category and searchText
  it('list_agent_memories accepts null category and searchText', async () => {
    await expect(invoke('list_agent_memories', { category: null, searchText: null })).resolves.not.toThrow();
  });
});

describe('invoke audit — no-param commands accept empty call', () => {
  const noParamCommands = [
    'get_all_chats',
    'get_all_folders',
    'load_settings',
    'get_custom_providers',
    'check_ollama_status',
    'get_local_ollama_models',
    'get_models',
    'mcp_get_servers',
    'mcp_list_connections',
    'get_fs_mcp_config',
    'clear_fs_audit_log',
    'get_image_styles',
    'get_mode_settings',
    'get_all_comparisons',
    'start_telegram_bot',
    'stop_telegram_bot',
    'get_telegram_status',
    'revoke_telegram_user',
    'get_all_presets',
    'get_all_categories',
    'get_all_snippets',
    'get_welcome_snippets',
    'get_all_templates',
    'list_knowledge_bases',
    'list_skills',
    'list_scheduled_tasks',
    'delete_all_agent_memories',
  ];

  for (const cmd of noParamCommands) {
    it(`${cmd} works without params`, async () => {
      await expect(invoke(cmd)).resolves.not.toThrow();
    });
  }
});
