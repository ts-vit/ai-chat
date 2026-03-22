export interface ModuleDefinition {
  id: string;
  name: string;
  description: string;
  type: "core" | "ui" | "ui-rust" | "rust";
  category: "core" | "settings" | "platform" | "ai" | "tools";
  // Rust crate name
  crate?: string;
  // npm package directory name (e.g. "uni-ssh-ui")
  package?: string;
  // Other module IDs that must be included
  dependencies?: string[];
  // Tauri commands to register in invoke_handler
  commands?: string[];
  // Command file template name (in templates/rust/commands/)
  commandTemplate?: string;
  // Tauri State type string
  stateSetup?: string;
  // State initialization code
  stateInit?: string;
  // Event bridge type: async = tokio::spawn, thread = std::thread::spawn
  eventBridge?: "async" | "thread";
  // Extra Rust use statements for lib.rs
  rustUses?: string[];
  // Settings page component name
  settingsComponent?: string;
  // Import source for settings component
  settingsImportFrom?: string;
  // Settings nav entry
  settingsNav?: { key: string; icon: string; labelKey: string };
  // i18n key paths to extract from Desktop i18n files
  i18nKeys?: string[];
}

// Core modules — always included, not selectable
export const CORE_MODULES: ModuleDefinition[] = [
  {
    id: "uni-common",
    name: "UNI Common",
    description: "Base utilities (generate_id, timestamps, truncation)",
    type: "core",
    category: "core",
    crate: "uni-common",
  },
  {
    id: "uni-settings",
    name: "UNI Settings",
    description: "File-based settings store",
    type: "core",
    category: "core",
    crate: "uni-settings",
  },
  {
    id: "uni-ui",
    name: "@uni/ui",
    description: "React components, theme, UniProvider, useSettings",
    type: "core",
    category: "core",
    package: "uni-ui",
  },
];

// Optional UI modules (type 1) — settings UI living in @uni/ui
export const UI_MODULES: ModuleDefinition[] = [
  {
    id: "openrouter",
    name: "OpenRouter",
    description: "OpenRouter models catalog & API keys",
    type: "ui",
    category: "settings",
    settingsComponent: "OpenRouterSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "openRouter",
      icon: "IconKey",
      labelKey: "settings.nav.openRouter",
    },
    i18nKeys: ["settings.openrouter"],
  },
  {
    id: "ollama",
    name: "Ollama",
    description: "Local Ollama models management",
    type: "ui",
    category: "settings",
    settingsComponent: "OllamaSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "ollama",
      icon: "IconServer",
      labelKey: "settings.nav.ollama",
    },
    i18nKeys: ["settings.ollama"],
  },
  {
    id: "generation",
    name: "Generation",
    description: "LLM generation parameters",
    type: "ui",
    category: "settings",
    settingsComponent: "GenerationSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "generation",
      icon: "IconAdjustments",
      labelKey: "settings.nav.generation",
    },
    i18nKeys: ["settings.generation"],
  },
  {
    id: "interface",
    name: "Interface",
    description: "UI settings (theme, font, language)",
    type: "ui",
    category: "settings",
    settingsComponent: "InterfaceSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "interface",
      icon: "IconPalette",
      labelKey: "settings.nav.interface",
    },
    i18nKeys: ["settings.interface"],
  },
  {
    id: "web-search",
    name: "Web Search",
    description: "Search provider & API keys",
    type: "ui",
    category: "settings",
    settingsComponent: "WebSearchSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "webSearch",
      icon: "IconWorldSearch",
      labelKey: "settings.nav.webSearch",
    },
    i18nKeys: ["settings.webSearch"],
  },
  {
    id: "budget",
    name: "Budget",
    description: "Budget limits per plan & global",
    type: "ui",
    category: "settings",
    settingsComponent: "BudgetSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "budget",
      icon: "IconCurrencyDollar",
      labelKey: "budget.title",
    },
    i18nKeys: ["budget"],
  },
];

// Optional UI+Rust modules (type 2)
export const UI_RUST_MODULES: ModuleDefinition[] = [
  {
    id: "ssh-tunnel",
    name: "SSH Tunnel (VPN)",
    description: "SOCKS5 proxy through SSH tunnel",
    type: "ui-rust",
    category: "platform",
    crate: "uni-ssh",
    package: "uni-ssh-ui",
    dependencies: [],
    commands: [
      "ssh_tunnel_connect",
      "ssh_tunnel_disconnect",
      "ssh_tunnel_status",
      "ssh_remove_known_host",
    ],
    commandTemplate: "ssh_tunnel",
    stateSetup: "Arc<uni_ssh::SshTunnelManager>",
    stateInit: "Arc::new(uni_ssh::SshTunnelManager::new())",
    eventBridge: "async",
    rustUses: ["use uni_ssh::SshTunnelManager"],
    settingsComponent: "SshTunnelSettings",
    settingsImportFrom: "@uni/ssh-ui",
    settingsNav: {
      key: "ssh",
      icon: "IconNetwork",
      labelKey: "settings.nav.vpn",
    },
    i18nKeys: ["settings.vpn"],
  },
  {
    id: "terminal",
    name: "Terminal",
    description: "Built-in terminal with PTY and tabs",
    type: "ui-rust",
    category: "tools",
    crate: "uni-terminal",
    package: "uni-terminal-ui",
    dependencies: ["uni-http"],
    commands: [
      "terminal_create",
      "terminal_write",
      "terminal_resize",
      "terminal_kill",
      "get_current_proxy_url",
    ],
    commandTemplate: "terminal",
    stateSetup: "std::sync::Mutex<uni_terminal::TerminalManager>",
    eventBridge: "thread",
    rustUses: ["use uni_terminal::TerminalManager"],
    settingsComponent: "TerminalSettings",
    settingsImportFrom: "@uni/ui",
    settingsNav: {
      key: "terminal-settings",
      icon: "IconTerminal2",
      labelKey: "settings.nav.terminal",
    },
    i18nKeys: ["terminal"],
  },
];

// Optional Rust-only modules (type 3)
export const RUST_MODULES: ModuleDefinition[] = [
  {
    id: "uni-http",
    name: "HTTP Client",
    description: "HTTP client with proxy support",
    type: "rust",
    category: "ai",
    crate: "uni-http",
    dependencies: [],
  },
  {
    id: "uni-llm",
    name: "LLM",
    description: "LLM providers & streaming",
    type: "rust",
    category: "ai",
    crate: "uni-llm",
    dependencies: ["uni-http"],
  },
  {
    id: "uni-embedding",
    name: "Embeddings",
    description: "Text embedding providers",
    type: "rust",
    category: "ai",
    crate: "uni-embedding",
    dependencies: ["uni-http"],
  },
  {
    id: "uni-search",
    name: "Search",
    description: "Vector + FTS search, chunker",
    type: "rust",
    category: "ai",
    crate: "uni-search",
    dependencies: [],
  },
  {
    id: "uni-audio",
    name: "Audio",
    description: "STT & TTS",
    type: "rust",
    category: "ai",
    crate: "uni-audio",
    dependencies: ["uni-http"],
  },
  {
    id: "uni-python",
    name: "Python",
    description: "Managed Python runtime",
    type: "rust",
    category: "tools",
    crate: "uni-python",
    dependencies: [],
  },
  {
    id: "uni-converter",
    name: "Converter",
    description: "Document → Markdown",
    type: "rust",
    category: "tools",
    crate: "uni-converter",
    dependencies: ["uni-http"],
  },
];

export const ALL_MODULES: ModuleDefinition[] = [
  ...CORE_MODULES,
  ...UI_MODULES,
  ...UI_RUST_MODULES,
  ...RUST_MODULES,
];

/** Resolve transitive dependencies, return full list of selected module IDs */
export function resolveModuleDeps(
  selectedIds: string[],
  allModules: ModuleDefinition[] = ALL_MODULES,
): string[] {
  const resolved = new Set(selectedIds);
  let changed = true;
  while (changed) {
    changed = false;
    for (const id of [...resolved]) {
      const mod = allModules.find((m) => m.id === id);
      if (mod?.dependencies) {
        for (const dep of mod.dependencies) {
          if (!resolved.has(dep)) {
            resolved.add(dep);
            changed = true;
          }
        }
      }
    }
  }
  return [...resolved];
}

/** Get ModuleDefinition objects for resolved IDs */
export function getModules(
  ids: string[],
  allModules: ModuleDefinition[] = ALL_MODULES,
): ModuleDefinition[] {
  return ids
    .map((id) => allModules.find((m) => m.id === id))
    .filter((m): m is ModuleDefinition => m !== undefined);
}
