import path from "path";
import { fileURLToPath } from "url";
import fs from "fs-extra";
import Handlebars from "handlebars";
import ora from "ora";
import type { AppConfig } from "./prompts.js";
import {
  ALL_MODULES,
  resolveModuleDeps,
  getModules,
  type ModuleDefinition,
} from "./modules.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Register helpers
Handlebars.registerHelper("snakeCase", (str: string) =>
  str.replace(/-/g, "_"),
);

export async function generateApp(config: AppConfig) {
  const spinner = ora("Creating app structure...").start();

  const isExternal = !!config.output;
  const rootDir = isExternal ? null : findMonorepoRoot();
  const appDir = isExternal
    ? path.resolve(config.output!)
    : path.join(rootDir!, "apps", config.name);

  if (await fs.pathExists(appDir)) {
    spinner.fail(
      isExternal
        ? `Directory ${appDir} already exists`
        : `Directory apps/${config.name} already exists`,
    );
    process.exit(1);
  }

  // Resolve module dependencies
  const resolvedIds = resolveModuleDeps(config.modules, ALL_MODULES);
  const selectedModules = getModules(resolvedIds, ALL_MODULES);

  // Log auto-resolved deps
  const autoResolved = resolvedIds.filter(
    (id) => !config.modules.includes(id),
  );
  if (autoResolved.length > 0) {
    spinner.info(`Auto-included dependencies: ${autoResolved.join(", ")}`);
    spinner.start("Creating app structure...");
  }

  // Computed flags
  const hasSshTunnel = resolvedIds.includes("ssh-tunnel");
  const hasTerminal = resolvedIds.includes("terminal");
  const hasDialogPlugin = hasSshTunnel;
  const hasServices = hasTerminal;
  const hasHttpClient = hasTerminal || resolvedIds.includes("uni-http");
  const settingsModules = selectedModules.filter((m) => m.settingsComponent);
  const hasSettingsPage = settingsModules.length > 0;
  const hasHeaderControls = hasSettingsPage || hasTerminal;

  // Crates for Cargo.toml (exclude core — they're always present)
  const crates = selectedModules
    .filter((m) => m.crate && m.type !== "core")
    .map((m) => ({
      name: m.crate!,
    }));

  // npm packages for package.json
  const packages = selectedModules
    .filter((m) => m.package)
    .map((m) => ({
      name: `@uni-fw/${m.package!.replace("uni-", "")}`,
    }));

  // Settings page data
  const settingsImports = dedupeImports(
    settingsModules.map((m) => ({
      component: m.settingsComponent!,
      from: m.settingsImportFrom!,
    })),
  );

  const settingsNavItems = settingsModules
    .filter((m) => m.settingsNav)
    .map((m) => ({
      key: m.settingsNav!.key,
      icon: m.settingsNav!.icon,
      labelKey: m.settingsNav!.labelKey,
      component: m.settingsComponent!,
    }));

  const settingsNavIcons = [
    ...new Set(settingsNavItems.map((item) => item.icon)),
  ];
  const firstSettingsKey = settingsNavItems[0]?.key ?? "";

  // Template data
  const data = {
    name: config.name,
    displayName: config.displayName,
    description: config.description,
    identifier: config.identifier,
    isExternal,

    // Module flags
    hasSshTunnel,
    hasTerminal,
    hasDialogPlugin,
    hasServices,
    hasHttpClient,
    hasSettingsPage,
    hasHeaderControls,
    hasHeaderIcons: hasSettingsPage || hasTerminal,
    hasTerminalAndSettings: hasTerminal && hasSettingsPage,

    // Collections
    crates,
    packages,
    settingsImports,
    settingsNavItems,
    settingsNavIcons,
    firstSettingsKey,
  };

  // Create directories
  const dirs = [
    "src",
    "src/components",
    "src/i18n/locales",
    "src-tauri/src/commands",
    "src-tauri/capabilities",
  ];
  if (hasServices) {
    dirs.push("src-tauri/src/services");
  }
  for (const dir of dirs) {
    await fs.mkdirp(path.join(appDir, dir));
  }

  // Generate files from templates
  // Works from both src/ (dev) and dist/ (published) — templates/ is sibling
  const templatesDir = path.resolve(__dirname, "../templates");

  const files: [string, string][] = [
    // Rust backend
    ["rust/Cargo.toml.hbs", "src-tauri/Cargo.toml"],
    ["rust/main.rs.hbs", "src-tauri/src/main.rs"],
    ["rust/lib.rs.hbs", "src-tauri/src/lib.rs"],
    ["rust/commands/mod.rs.hbs", "src-tauri/src/commands/mod.rs"],
    [
      "rust/commands/uni_settings.rs.hbs",
      "src-tauri/src/commands/uni_settings.rs",
    ],
    // Tauri config
    ["tauri/tauri.conf.json.hbs", "src-tauri/tauri.conf.json"],
    ["tauri/build.rs.hbs", "src-tauri/build.rs"],
    [
      "tauri/capabilities/default.json.hbs",
      "src-tauri/capabilities/default.json",
    ],
    // Frontend
    ["frontend/package.json.hbs", "package.json"],
    ["frontend/npmrc.hbs", ".npmrc"],
    ["frontend/index.html.hbs", "index.html"],
    ["frontend/vite.config.ts.hbs", "vite.config.ts"],
    ["frontend/tsconfig.json.hbs", "tsconfig.json"],
    ["frontend/tsconfig.node.json.hbs", "tsconfig.node.json"],
    ["frontend/vite-env.d.ts.hbs", "src/vite-env.d.ts"],
    ["frontend/main.tsx.hbs", "src/main.tsx"],
    ["frontend/App.tsx.hbs", "src/App.tsx"],
    ["frontend/App.css.hbs", "src/App.css"],
    ["frontend/i18n/i18n.ts.hbs", "src/i18n/i18n.ts"],
  ];

  // Conditional command templates
  if (hasSshTunnel) {
    files.push([
      "rust/commands/ssh_tunnel.rs.hbs",
      "src-tauri/src/commands/ssh_tunnel.rs",
    ]);
  }
  if (hasTerminal) {
    files.push([
      "rust/commands/terminal.rs.hbs",
      "src-tauri/src/commands/terminal.rs",
    ]);
  }
  if (hasServices) {
    files.push([
      "rust/services/mod.rs.hbs",
      "src-tauri/src/services/mod.rs",
    ]);
    files.push([
      "rust/services/http_client.rs.hbs",
      "src-tauri/src/services/http_client.rs",
    ]);
  }
  if (hasSettingsPage) {
    files.push([
      "frontend/SettingsPage.tsx.hbs",
      "src/components/SettingsPage.tsx",
    ]);
  }

  for (const [templatePath, outputPath] of files) {
    await renderTemplate(templatesDir, templatePath, appDir, outputPath, data);
  }

  // Generate i18n files programmatically
  spinner.text = "Generating i18n files...";
  if (rootDir) {
    await generateI18n(rootDir, appDir, config, selectedModules);
  } else {
    await generateI18nMinimal(appDir, config);
  }

  // Copy icons
  spinner.text = "Copying icons...";
  const appIcons = path.join(appDir, "src-tauri", "icons");
  if (rootDir) {
    const desktopIcons = path.join(
      rootDir,
      "apps",
      "desktop",
      "src-tauri",
      "icons",
    );
    if (await fs.pathExists(desktopIcons)) {
      await fs.copy(desktopIcons, appIcons, {
        filter: (src) => !src.includes("android") && !src.includes("ios"),
      });
    }
  } else {
    // External project — copy default icons from templates
    const defaultIconsDir = path.resolve(__dirname, "../templates/icons");
    if (await fs.pathExists(defaultIconsDir)) {
      await fs.mkdirp(appIcons);
      const iconFiles = await fs.readdir(defaultIconsDir);
      for (const file of iconFiles) {
        await fs.copyFile(
          path.join(defaultIconsDir, file),
          path.join(appIcons, file),
        );
      }
    }
  }

  if (isExternal) {
    // Create root Cargo.toml workspace
    const workspaceCargo = `[workspace]\nmembers = ["src-tauri"]\nresolver = "2"\n`;
    await fs.writeFile(path.join(appDir, "Cargo.toml"), workspaceCargo);

    // Create .gitignore
    const gitignore = `node_modules/\ndist/\ntarget/\n.env\n*.log\n`;
    await fs.writeFile(path.join(appDir, ".gitignore"), gitignore);

    spinner.succeed(`App created at ${appDir}/`);
  } else {
    // Add to workspace members in root Cargo.toml
    spinner.text = "Updating workspace...";
    await addToCargoWorkspace(rootDir!, `apps/${config.name}/src-tauri`);

    spinner.succeed(`App created at apps/${config.name}/`);
  }

  // Summary
  if (selectedModules.filter((m) => m.type !== "core").length > 0) {
    const nonCore = selectedModules.filter((m) => m.type !== "core");
    console.log(
      `  Modules: ${nonCore.map((m) => m.name).join(", ")}`,
    );
  }
}

/** Group imports by source, merge components */
function dedupeImports(
  imports: { component: string; from: string }[],
): { component: string; from: string }[] {
  const bySource = new Map<string, string[]>();
  for (const imp of imports) {
    const list = bySource.get(imp.from) ?? [];
    if (!list.includes(imp.component)) {
      list.push(imp.component);
    }
    bySource.set(imp.from, list);
  }
  return [...bySource.entries()].map(([from, components]) => ({
    component: components.join(", "),
    from,
  }));
}

/** Generate minimal i18n files for external (standalone) projects */
async function generateI18nMinimal(appDir: string, config: AppConfig) {
  const en = {
    app: { title: config.displayName, description: config.description },
    common: {
      save: "Save",
      cancel: "Cancel",
      delete: "Delete",
      loading: "Loading...",
      settings: "Settings",
    },
  };
  const ru = {
    app: { title: config.displayName, description: config.description },
    common: {
      save: "Сохранить",
      cancel: "Отмена",
      delete: "Удалить",
      loading: "Загрузка...",
      settings: "Настройки",
    },
  };
  await fs.writeJson(path.join(appDir, "src/i18n/locales/en.json"), en, {
    spaces: 2,
  });
  await fs.writeJson(path.join(appDir, "src/i18n/locales/ru.json"), ru, {
    spaces: 2,
  });
}

/** Generate i18n en.json and ru.json by extracting keys from Desktop i18n */
async function generateI18n(
  rootDir: string,
  appDir: string,
  config: AppConfig,
  selectedModules: ModuleDefinition[],
) {
  const desktopLocales = path.join(
    rootDir,
    "apps",
    "desktop",
    "src",
    "i18n",
    "locales",
  );

  let desktopEn: Record<string, unknown> = {};
  let desktopRu: Record<string, unknown> = {};
  try {
    desktopEn = await fs.readJson(path.join(desktopLocales, "en.json"));
    desktopRu = await fs.readJson(path.join(desktopLocales, "ru.json"));
  } catch {
    // Desktop i18n not found — use minimal defaults
  }

  // Base keys
  const en: Record<string, unknown> = {
    app: {
      title: config.displayName,
      description: config.description,
    },
    common: {
      save: "Save",
      cancel: "Cancel",
      delete: "Delete",
      loading: "Loading...",
      settings: "Settings",
    },
  };

  const ru: Record<string, unknown> = {
    app: {
      title: config.displayName,
      description: config.description,
    },
    common: {
      save: "Сохранить",
      cancel: "Отмена",
      delete: "Удалить",
      loading: "Загрузка...",
      settings: "Настройки",
    },
  };

  // Extract settings.nav keys for selected modules
  const settingsModules = selectedModules.filter((m) => m.settingsNav);
  if (settingsModules.length > 0) {
    const navEn: Record<string, unknown> = {};
    const navRu: Record<string, unknown> = {};
    for (const mod of settingsModules) {
      const navKey = mod.settingsNav!.labelKey; // e.g. "settings.nav.openRouter"
      const parts = navKey.split(".");
      const leafKey = parts[parts.length - 1];
      const enVal = getNestedValue(desktopEn, navKey);
      const ruVal = getNestedValue(desktopRu, navKey);
      if (enVal !== undefined) navEn[leafKey] = enVal;
      if (ruVal !== undefined) navRu[leafKey] = ruVal;
    }
    ensureNested(en, "settings.nav", navEn);
    ensureNested(ru, "settings.nav", navRu);
  }

  // Extract i18n keys for each module
  for (const mod of selectedModules) {
    if (!mod.i18nKeys) continue;
    for (const keyPath of mod.i18nKeys) {
      const enVal = getNestedValue(desktopEn, keyPath);
      const ruVal = getNestedValue(desktopRu, keyPath);
      if (enVal !== undefined) setNestedValue(en, keyPath, enVal);
      if (ruVal !== undefined) setNestedValue(ru, keyPath, ruVal);
    }
  }

  await fs.writeJson(
    path.join(appDir, "src/i18n/locales/en.json"),
    en,
    { spaces: 2 },
  );
  await fs.writeJson(
    path.join(appDir, "src/i18n/locales/ru.json"),
    ru,
    { spaces: 2 },
  );
}

/** Get a nested value by dot-separated path */
function getNestedValue(obj: Record<string, unknown>, keyPath: string): unknown {
  const parts = keyPath.split(".");
  let current: unknown = obj;
  for (const part of parts) {
    if (current === null || current === undefined || typeof current !== "object") {
      return undefined;
    }
    current = (current as Record<string, unknown>)[part];
  }
  return current;
}

/** Set a nested value by dot-separated path */
function setNestedValue(
  obj: Record<string, unknown>,
  keyPath: string,
  value: unknown,
) {
  const parts = keyPath.split(".");
  let current = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!(parts[i] in current) || typeof current[parts[i]] !== "object") {
      current[parts[i]] = {};
    }
    current = current[parts[i]] as Record<string, unknown>;
  }
  current[parts[parts.length - 1]] = value;
}

/** Ensure nested path exists and merge an object into it */
function ensureNested(
  obj: Record<string, unknown>,
  keyPath: string,
  value: Record<string, unknown>,
) {
  const existing = getNestedValue(obj, keyPath);
  if (existing && typeof existing === "object") {
    setNestedValue(obj, keyPath, { ...(existing as Record<string, unknown>), ...value });
  } else {
    setNestedValue(obj, keyPath, value);
  }
}

async function renderTemplate(
  templatesDir: string,
  templatePath: string,
  appDir: string,
  outputPath: string,
  data: Record<string, unknown>,
) {
  const templateSrc = await fs.readFile(
    path.join(templatesDir, templatePath),
    "utf-8",
  );
  const template = Handlebars.compile(templateSrc);
  const result = template(data);
  await fs.writeFile(path.join(appDir, outputPath), result);
}

function findMonorepoRoot(): string {
  let dir = process.cwd();
  while (dir !== path.dirname(dir)) {
    if (
      fs.existsSync(path.join(dir, "Cargo.toml")) &&
      fs.existsSync(path.join(dir, "crates"))
    ) {
      return dir;
    }
    dir = path.dirname(dir);
  }
  throw new Error(
    "Could not find monorepo root. Run from within the ai-chat monorepo, or use --output for standalone projects.",
  );
}

async function addToCargoWorkspace(rootDir: string, member: string) {
  const cargoPath = path.join(rootDir, "Cargo.toml");
  let content = await fs.readFile(cargoPath, "utf-8");
  const memberLine = `    "${member}",`;
  if (!content.includes(memberLine)) {
    // Insert before the closing bracket of members array
    content = content.replace(
      /(members = \[[\s\S]*?)(^\])/m,
      `$1${memberLine}\n$2`,
    );
    await fs.writeFile(cargoPath, content);
  }
}
