#!/usr/bin/env node
import chalk from "chalk";
import { askQuestions, type AppConfig } from "./prompts.js";
import { generateApp } from "./generator.js";

function parseArgs(): Partial<AppConfig> | null {
  const args = process.argv.slice(2);
  const result: Record<string, string> = {};
  let modules: string[] | undefined;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--name" && args[i + 1]) result.name = args[++i];
    else if (args[i] === "--display-name" && args[i + 1])
      result.displayName = args[++i];
    else if (args[i] === "--description" && args[i + 1])
      result.description = args[++i];
    else if (args[i] === "--modules" && args[i + 1]) {
      modules = args[++i].split(",").filter(Boolean);
    }
  }
  if (Object.keys(result).length === 0 && !modules) return null;
  return { ...result, modules } as Partial<AppConfig>;
}

async function main() {
  console.log(chalk.bold("\n  Create UNI App\n"));

  const cliArgs = parseArgs();
  let config: AppConfig;

  if (cliArgs?.name) {
    // Non-interactive mode
    const name = cliArgs.name;
    config = {
      name,
      displayName:
        cliArgs.displayName ??
        name
          .split("-")
          .map((w) => w[0].toUpperCase() + w.slice(1))
          .join(" "),
      description: cliArgs.description ?? "A UNI Framework application",
      identifier: `com.uni.${name}`,
      modules: cliArgs.modules ?? [],
    };
  } else {
    config = await askQuestions();
  }

  await generateApp(config);

  console.log(chalk.green("\n  App created successfully!"));
  console.log(chalk.dim(`\nNext steps:`));
  console.log(chalk.dim(`  cd apps/${config.name}`));
  console.log(chalk.dim(`  npm install`));
  console.log(chalk.dim(`  npm run dev\n`));
}

main().catch(console.error);
