import js from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import svelte from "eslint-plugin-svelte";
import ts from "typescript-eslint";

// Based on https://sveltejs.github.io/eslint-plugin-svelte/user-guide/ (TypeScript).
export default defineConfig(
  // .claude/ holds local agent worktrees and reference/ the design references (the demo,
  // the vector Mochi script): neither is îlot code.
  globalIgnores(["dist/", "target/", "src-tauri/gen/", ".claude/", "reference/"]),
  js.configs.recommended,
  ts.configs.recommended,
  svelte.configs.recommended,
  {
    languageOptions: {
      parserOptions: {
        extraFileExtensions: [".svelte"],
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: {
      parserOptions: {
        projectService: true,
        parser: ts.parser,
      },
    },
    // TypeScript already reports undefined names, with the DOM types (typescript-eslint FAQ).
    rules: {
      "no-undef": "off",
    },
  },
  {
    // Build scripts run in Node.
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { process: "readonly" } },
  },
);
