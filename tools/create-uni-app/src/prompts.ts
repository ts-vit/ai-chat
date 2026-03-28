import inquirer from "inquirer";

export interface AppConfig {
  name: string;
  displayName: string;
  description: string;
  identifier: string;
  modules: string[];
  output?: string;
}

export async function askQuestions(): Promise<AppConfig> {
  const base = await inquirer.prompt([
    {
      type: "input",
      name: "name",
      message: "App name (kebab-case):",
      validate: (v: string) =>
        /^[a-z][a-z0-9-]*$/.test(v) ||
        "Use kebab-case (lowercase letters, digits, hyphens)",
      default: "my-app",
    },
    {
      type: "input",
      name: "displayName",
      message: "Display name:",
      default: (a: { name: string }) =>
        a.name
          .split("-")
          .map((w: string) => w[0].toUpperCase() + w.slice(1))
          .join(" "),
    },
    {
      type: "input",
      name: "description",
      message: "Description:",
      default: "A UNI Framework application",
    },
  ]);

  const modulesAnswers = await inquirer.prompt([
    {
      type: "checkbox",
      name: "settingsModules",
      message: "Settings modules:",
      choices: [
        { name: "OpenRouter — models catalog & API keys", value: "openrouter" },
        { name: "Ollama — local models management", value: "ollama" },
        { name: "Generation — LLM parameters", value: "generation" },
        { name: "Interface — UI settings (theme, font)", value: "interface" },
        { name: "Web Search — search provider settings", value: "web-search" },
        { name: "Budget — spending limits", value: "budget" },
      ],
    },
    {
      type: "checkbox",
      name: "platformModules",
      message: "Platform modules:",
      choices: [
        {
          name: "SSH Tunnel — VPN via SSH (SOCKS5 proxy)",
          value: "ssh-tunnel",
        },
        {
          name: "Terminal — built-in terminal with PTY",
          value: "terminal",
        },
      ],
    },
    {
      type: "checkbox",
      name: "rustModules",
      message: "Rust crates:",
      choices: [
        { name: "HTTP Client — with proxy support", value: "uni-http" },
        { name: "LLM — providers & streaming", value: "uni-llm" },
        { name: "Embeddings — text embedding", value: "uni-embedding" },
        { name: "Search — vector + FTS", value: "uni-search" },
        { name: "Audio — STT & TTS", value: "uni-audio" },
        { name: "Python — managed runtime", value: "uni-python" },
        { name: "Converter — document → markdown", value: "uni-converter" },
      ],
    },
  ]);

  const modules: string[] = [
    ...modulesAnswers.settingsModules,
    ...modulesAnswers.platformModules,
    ...modulesAnswers.rustModules,
  ];

  return {
    ...base,
    identifier: `com.uni.${base.name}`,
    modules,
  };
}
