export interface McpPreset {
    name: string;
    command: string;
    args: string[];
    env: Record<string, string>;
    description: string;
    descriptionEn: string;
    requiresEnv: string[];
}

export const MCP_PRESETS: McpPreset[] = [
    {
        name: "Brave Search",
        command: "npx",
        args: ["-y", "@anthropic-ai/mcp-server-brave-search"],
        env: { BRAVE_API_KEY: "" },
        description: "Поиск в интернете через Brave Search API",
        descriptionEn: "Web search via Brave Search API",
        requiresEnv: ["BRAVE_API_KEY"],
    },
    {
        name: "GitHub",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-github"],
        env: { GITHUB_PERSONAL_ACCESS_TOKEN: "" },
        description: "Работа с GitHub репозиториями",
        descriptionEn: "Work with GitHub repositories",
        requiresEnv: ["GITHUB_PERSONAL_ACCESS_TOKEN"],
    },
    {
        name: "SQLite",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-sqlite", ""],
        env: {},
        description: "Работа с SQLite базами данных",
        descriptionEn: "Work with SQLite databases",
        requiresEnv: [],
    },
    {
        name: "Filesystem",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-filesystem", ""],
        env: {},
        description: "Доступ к файлам на диске",
        descriptionEn: "Access files on disk",
        requiresEnv: [],
    },
    {
        name: "Puppeteer",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-puppeteer"],
        env: {},
        description: "Управление браузером для скрапинга",
        descriptionEn: "Browser automation for scraping",
        requiresEnv: [],
    },
];
