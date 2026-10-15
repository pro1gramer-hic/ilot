// Drafts a new language of îlot's interface from the French catalogs, with DeepL
// (docs/LANGUES.md). Usage, from the repository's root:
//
//   $env:DEEPL_API_KEY = "…"; npm.cmd run i18n:translate -- de
//
// Reads each src/lib/i18n/fr/<part>.ts and sends only its texts to DeepL: strings, and the
// templates of simple functions (`(name: string) => \`Répondre à ${name}\``). The
// `${…}` and the proper names (îlot, Mochi, Claude…) travel inside tags DeepL leaves alone
// (`tag_handling: "xml"`, `ignore_tags`). A function with more logic (a plural, a choice)
// is copied in French under `// TODO(<lang>): relire le pluriel (texte français gardé)`:
// plural rules differ from one language to the other and need a person. Writes
// src/lib/i18n/<lang>/<part>.ts, typed `Catalog["<part>"]` like the English ones, and an
// index.ts. The key is read from the environment only: never written, never printed.
//
// DeepL: https://developers.deepl.com/api-reference/translate (POST /v2/translate),
// https://developers.deepl.com/docs/getting-started/auth (`DeepL-Auth-Key`, `:fx` keys go to
// api-free.deepl.com), https://developers.deepl.com/docs/xml-and-html-handling/xml
// (`ignore_tags`), https://developers.deepl.com/docs/resources/usage-limits (128 KiB).

import { Buffer } from "node:buffer";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { setTimeout as wait } from "node:timers/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

export const FREE_ENDPOINT = "https://api-free.deepl.com/v2/translate";
export const PRO_ENDPOINT = "https://api.deepl.com/v2/translate";

/** Names that stay as they are in every language, the longest first. */
export const PROPER_NAMES = [
  "Gestionnaire d'identification Windows",
  "Windows Terminal",
  "Claude Code",
  "Claude Desktop",
  "Gemini CLI",
  "Copilot CLI",
  "Kiro CLI",
  "IBM Bob",
  "LM Studio",
  "Docker Hub",
  "WhatsApp Web",
  "VS Code",
  "DeepL API Free",
  "DeepL API Pro",
  "îlot",
  "Mochi",
  "Claude",
  "Anthropic",
  "Gemini",
  "Google",
  "OpenAI",
  "Codex",
  "Ollama",
  "WhatsApp",
  "Discord",
  "Snapchat",
  "Spotify",
  "YouTube",
  "Crunchyroll",
  "Windows",
  "DeepL",
  "GitHub",
  "GitLab",
  "Vercel",
  "Cloudflare",
  "Dokploy",
  "Stripe",
  "n8n",
  "Resend",
  "Notion",
  "Cal.com",
  "Cursor",
  "Copilot",
  "OpenCode",
  "Amp",
  "Kiro",
  "Antigravity",
  "Outlook",
  "WSL",
  "PowerShell",
  "Ctrl",
  "Alt",
].sort((a, b) => b.length - a.length);

/** The tag DeepL leaves alone. */
const TAG = "x";
/** DeepL takes 128 KiB per request: stay well under, and at 50 texts. */
const MAX_BATCH_BYTES = 96 * 1024;
const MAX_BATCH_TEXTS = 50;
/** Statuses worth another try, after a pause that doubles (usage-limits). */
const RETRY_STATUSES = [429, 500, 502, 503, 504, 529];
const RETRIES = 4;

/** `:fx` keys belong to DeepL API Free, which has its own host. */
export function endpointFor(key) {
  return key.trim().endsWith(":fx") ? FREE_ENDPOINT : PRO_ENDPOINT;
}

/** A folder name for the language: `de`, `pt-br`. */
export function validLang(lang) {
  return /^[a-z]{2,3}(-[a-z0-9]{2,4})?$/.test(lang);
}

/** `pt-br` → `ptBr`: the name of the catalog in index.ts. */
export function langIdentifier(lang) {
  return lang.replace(/-([a-z0-9])/g, (_, c) => c.toUpperCase());
}

export function escapeXml(text) {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

export function unescapeXml(text) {
  return text
    .replace(/&#x([0-9a-f]+);/gi, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
    .replace(/&#(\d+);/g, (_, dec) => String.fromCodePoint(Number(dec)))
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'")
    .replaceAll("&amp;", "&");
}

const NAME_PATTERN = new RegExp(
  `(?<![\\p{L}\\p{N}])(${PROPER_NAMES.map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})(?![\\p{L}\\p{N}])`,
  "gu",
);

/**
 * A text as DeepL gets it. `parts` alternate words (`{ text }`) and what must come back
 * untouched (`{ keep }`, the source of a `${…}`). Proper names are kept too. Returns the XML
 * and the list of kept pieces, numbered by their tag.
 */
export function protect(parts) {
  const kept = [];
  const tag = (raw) => {
    kept.push(raw);
    return `<${TAG} i="${kept.length - 1}">${escapeXml(raw)}</${TAG}>`;
  };
  let xml = "";
  for (const part of parts) {
    if ("keep" in part) {
      xml += tag(part.keep);
      continue;
    }
    let at = 0;
    for (const match of part.text.matchAll(NAME_PATTERN)) {
      xml += escapeXml(part.text.slice(at, match.index)) + tag(match[0]);
      at = match.index + match[0].length;
    }
    xml += escapeXml(part.text.slice(at));
  }
  return { xml, kept };
}

/**
 * DeepL's answer back to parts: each tag gives back exactly what was kept under its number
 * (whatever DeepL wrote inside). Null when a tag went missing, came twice or is unknown: the
 * French text is then kept.
 */
export function restore(xml, kept, wasName) {
  const parts = [];
  const seen = new Set();
  const pattern = new RegExp(`<${TAG} i="(\\d+)"\\s*>[\\s\\S]*?</${TAG}>|<${TAG} i="(\\d+)"\\s*/>`, "g");
  let at = 0;
  for (const match of xml.matchAll(pattern)) {
    const index = Number(match[1] ?? match[2]);
    if (!(index in kept) || seen.has(index)) return null;
    seen.add(index);
    const before = unescapeXml(xml.slice(at, match.index));
    if (before) parts.push({ text: before });
    parts.push(wasName(index) ? { text: kept[index] } : { keep: kept[index] });
    at = match.index + match[0].length;
  }
  const rest = xml.slice(at);
  if (rest.includes(`<${TAG}`) || seen.size !== kept.length) return null;
  if (rest) parts.push({ text: unescapeXml(rest) });
  return merge(parts);
}

/** Neighbouring words become one piece. */
function merge(parts) {
  const merged = [];
  for (const part of parts) {
    const last = merged.at(-1);
    if ("text" in part && last && "text" in last) last.text += part.text;
    else merged.push({ ...part });
  }
  return merged;
}

/** Parts as a TypeScript literal: a string when nothing is kept, else a template. */
export function literal(parts) {
  if (parts.every((part) => "text" in part)) return JSON.stringify(parts.map((part) => part.text).join(""));
  const body = parts
    .map((part) => ("keep" in part ? part.keep : part.text.replaceAll("\\", "\\\\").replaceAll("`", "\\`").replaceAll("${", "\\${")))
    .join("");
  return `\`${body}\``;
}

const hasLetters = (text) => /\p{L}/u.test(text);

/** The words of a string or a template, with each `${…}` kept as its source. */
function partsOf(node, source) {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return [{ text: node.text }];
  const parts = [{ text: node.head.text }];
  for (const span of node.templateSpans) {
    parts.push({ keep: "${" + source.slice(span.expression.getStart(), span.expression.getEnd()) + "}" });
    parts.push({ text: span.literal.text });
  }
  return merge(parts.filter((part) => !("text" in part) || part.text !== ""));
}

const isText = (node) => ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node) || ts.isTemplateExpression(node);

function unwrap(node) {
  let inner = node;
  while (ts.isParenthesizedExpression(inner) || ts.isSatisfiesExpression(inner) || ts.isAsExpression(inner)) inner = inner.expression;
  return inner;
}

/** A function DeepL can take: it returns a string or a template whose `${…}` are its own
 * parameters (or their properties), nothing else. */
function simpleBody(fn) {
  if (!ts.isArrowFunction(fn) || ts.isBlock(fn.body)) return null;
  const body = unwrap(fn.body);
  if (!isText(body)) return null;
  const params = new Set(fn.parameters.map((p) => (ts.isIdentifier(p.name) ? p.name.text : "")));
  const rooted = (expression) => {
    let root = expression;
    while (ts.isPropertyAccessExpression(root)) root = root.expression;
    return ts.isIdentifier(root) && params.has(root.text);
  };
  if (ts.isTemplateExpression(body) && !body.templateSpans.every((span) => rooted(span.expression))) return null;
  return body;
}

/**
 * What a French catalog holds: the exported object, its texts (`units`: where they are and
 * their parts) and the functions to copy as they are (`kept`: where their property starts).
 */
export function extract(source, fileName = "part.ts") {
  const file = ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const units = [];
  const kept = [];
  let exported = null;
  const visit = (node, owner) => {
    const value = unwrap(node);
    if (isText(value)) {
      const parts = partsOf(value, source);
      if (parts.some((part) => "text" in part && hasLetters(part.text))) units.push({ start: value.getStart(), end: value.getEnd(), parts });
    } else if (ts.isObjectLiteralExpression(value)) {
      for (const property of value.properties) {
        if (ts.isPropertyAssignment(property)) visit(property.initializer, property);
      }
    } else if (ts.isArrayLiteralExpression(value)) {
      for (const element of value.elements) visit(element, owner);
    } else if (ts.isArrowFunction(value) || ts.isFunctionExpression(value)) {
      const body = simpleBody(value);
      if (body) visit(body, owner);
      else kept.push({ start: owner.getStart(file) });
    }
  };
  for (const statement of file.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    if (!statement.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (ts.isIdentifier(declaration.name) && declaration.initializer && ts.isObjectLiteralExpression(declaration.initializer)) {
        exported = { name: declaration.name.text, nameEnd: declaration.name.getEnd() };
        visit(declaration.initializer, declaration);
      }
    }
  }
  if (!exported) throw new Error(`${fileName} : aucun objet exporté (export const … = { … })`);
  return { exported, units, kept };
}

/** The leading `//` comment of a file, and where the code starts after it. */
function headerEnd(source) {
  const lines = source.split("\n");
  let index = 0;
  while (index < lines.length && lines[index].startsWith("//")) index += 1;
  while (index < lines.length && lines[index].trim() === "") index += 1;
  return lines.slice(0, index).join("\n").length + (index > 0 ? 1 : 0);
}

/**
 * The new language's file of a part: the French source with each text replaced by its
 * translation (`translated`: by unit, its new literal, or null to keep the French one), a
 * TODO above each function kept, the export typed `Catalog["<part>"]`, a new header.
 */
export function render({ source, part, lang, extracted, translated }) {
  const edits = [];
  extracted.units.forEach((unit, index) => {
    const replacement = translated[index];
    if (replacement !== null) edits.push({ at: unit.start, end: unit.end, text: replacement });
  });
  const todo = (start, why) => {
    const lineStart = source.lastIndexOf("\n", start - 1) + 1;
    const indent = source.slice(lineStart).match(/^\s*/)[0];
    edits.push({ at: lineStart, end: lineStart, text: `${indent}// TODO(${lang}): ${why}\n` });
  };
  for (const { start } of extracted.kept) todo(start, "relire le pluriel (texte français gardé)");
  extracted.units.forEach((unit, index) => {
    if (translated[index] === null) todo(unit.start, "traduction à refaire, balises perdues (texte français gardé)");
  });
  edits.push({ at: extracted.exported.nameEnd, end: extracted.exported.nameEnd, text: `: Catalog["${part}"]` });
  const codeStart = headerEnd(source);
  edits.push({
    at: 0,
    end: codeStart,
    text: `// ${part}, in "${lang}": the same keys as ../fr/${part}.ts, translated by DeepL with\n// scripts/translate-catalog.mjs. To review before use (docs/LANGUES.md).\n\nimport type { Catalog } from "../fr";\n\n`,
  });
  let out = source;
  for (const edit of edits.sort((a, b) => b.at - a.at || b.end - a.end)) out = out.slice(0, edit.at) + edit.text + out.slice(edit.end);
  return out;
}

/** The parts of the French catalog, in its order (src/lib/i18n/fr/index.ts). */
export function partsOfIndex(source) {
  const file = ts.createSourceFile("index.ts", source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  for (const statement of file.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      const value = declaration.initializer;
      if (value && ts.isObjectLiteralExpression(value)) {
        return value.properties.filter(ts.isShorthandPropertyAssignment).map((property) => property.name.text);
      }
    }
  }
  throw new Error("fr/index.ts : liste des parties introuvable");
}

export function renderIndex(lang, parts) {
  const name = langIdentifier(lang);
  return [
    `// The "${lang}" texts: the same keys as the French catalog, translated by DeepL with`,
    "// scripts/translate-catalog.mjs. To review before use (docs/LANGUES.md).",
    "",
    'import type { Catalog } from "../fr";',
    ...parts.map((part) => `import { ${part} } from "./${part}";`),
    "",
    `export const ${name}: Catalog = {`,
    ...parts.map((part) => `  ${part},`),
    "};",
    "",
  ].join("\n");
}

/** Texts in batches DeepL accepts. */
export function batches(texts) {
  const list = [];
  let current = [];
  let bytes = 0;
  for (const text of texts) {
    const size = Buffer.byteLength(text, "utf8") + 8;
    if (current.length && (current.length >= MAX_BATCH_TEXTS || bytes + size > MAX_BATCH_BYTES)) {
      list.push(current);
      current = [];
      bytes = 0;
    }
    current.push(text);
    bytes += size;
  }
  if (current.length) list.push(current);
  return list;
}

/** Sends one batch; waits and tries again when DeepL asks to, stops on a refusal. */
export async function requestBatch({ texts, target, key, fetch, sleep }) {
  const body = JSON.stringify({
    text: texts,
    source_lang: "FR",
    target_lang: target,
    tag_handling: "xml",
    ignore_tags: [TAG],
    preserve_formatting: true,
    formality: "prefer_less",
    context: "Interface texts of îlot, a small Windows app at the top of the screen: buttons, labels and short sentences, in a friendly informal tone.",
  });
  for (let attempt = 0; ; attempt += 1) {
    const response = await fetch(endpointFor(key), {
      method: "POST",
      headers: { Authorization: `DeepL-Auth-Key ${key.trim()}`, "Content-Type": "application/json" },
      body,
      redirect: "error",
    });
    if (response.ok) {
      const data = await response.json();
      const translations = data?.translations;
      if (!Array.isArray(translations) || translations.length !== texts.length) throw new Error("Réponse inattendue de DeepL.");
      return translations.map((item) => String(item?.text ?? ""));
    }
    if (RETRY_STATUSES.includes(response.status) && attempt < RETRIES) {
      await sleep(1000 * 2 ** attempt);
      continue;
    }
    if (response.status === 403) throw new Error("DeepL refuse la clé (403). Vérifie DEEPL_API_KEY.");
    if (response.status === 456) throw new Error("Quota DeepL atteint (456).");
    throw new Error(`DeepL a répondu ${response.status}.`);
  }
}

/**
 * Translates every part of the French catalog into `lang` and writes the files. Nothing is
 * written when a request fails. Returns what was done.
 */
export async function translateCatalog({ root, lang, target = lang.toUpperCase(), key, fetch, sleep, force = false, log = () => {} }) {
  if (!validLang(lang)) throw new Error(`Langue invalide : « ${lang} » (exemple : de, es, pt-br).`);
  if (lang === "fr") throw new Error("Le français est la référence : rien à traduire.");
  if (!key || !key.trim()) throw new Error("Clé absente : mets-la dans la variable d'environnement DEEPL_API_KEY.");
  const i18n = join(root, "src", "lib", "i18n");
  const out = join(i18n, lang);
  if (existsSync(out) && !force) throw new Error(`${out} existe déjà : ajoute --force pour le remplacer.`);
  const parts = partsOfIndex(readFileSync(join(i18n, "fr", "index.ts"), "utf8"));
  const files = parts.map((part) => {
    const source = readFileSync(join(i18n, "fr", `${part}.ts`), "utf8");
    const extracted = extract(source, `${part}.ts`);
    const protectedUnits = extracted.units.map((unit) => {
      // Spaces around a text come back as they were: DeepL may drop them.
      const first = unit.parts[0];
      const last = unit.parts.at(-1);
      const lead = "text" in first ? first.text.match(/^\s*/)[0] : "";
      const trail = "text" in last ? last.text.match(/\s*$/)[0] : "";
      const inner = unit.parts.map((p, i) => {
        if (!("text" in p)) return p;
        let text = p.text;
        if (i === 0) text = text.slice(lead.length);
        if (i === unit.parts.length - 1) text = text.slice(0, text.length - trail.length);
        return { text };
      });
      return { ...protect(inner), lead, trail };
    });
    return { part, source, extracted, protectedUnits };
  });
  const unique = [...new Set(files.flatMap((file) => file.protectedUnits.map((unit) => unit.xml)))];
  const answers = new Map();
  for (const batch of batches(unique)) {
    const translated = await requestBatch({ texts: batch, target, key, fetch, sleep });
    batch.forEach((xml, index) => answers.set(xml, translated[index]));
    log(`${answers.size} / ${unique.length} textes traduits`);
  }
  let todos = 0;
  let fallbacks = 0;
  const written = files.map(({ part, source, extracted, protectedUnits }) => {
    const translated = protectedUnits.map((unit) => {
      const isName = (index) => !unit.kept[index].startsWith("${");
      const parts = restore(answers.get(unit.xml) ?? "", unit.kept, isName);
      if (!parts) {
        fallbacks += 1;
        return null;
      }
      return literal(merge([{ text: unit.lead }, ...parts, { text: unit.trail }].filter((p) => !("text" in p) || p.text !== "")));
    });
    todos += extracted.kept.length;
    return { part, text: render({ source, part, lang, extracted, translated }) };
  });
  mkdirSync(out, { recursive: true });
  for (const { part, text } of written) writeFileSync(join(out, `${part}.ts`), text, "utf8");
  writeFileSync(join(out, "index.ts"), renderIndex(lang, parts), "utf8");
  return { folder: out, parts: parts.length, texts: unique.length, todos, fallbacks };
}

async function main() {
  const args = process.argv.slice(2);
  const lang = args.find((arg) => !arg.startsWith("--"));
  const target = args.find((arg) => arg.startsWith("--target="))?.slice("--target=".length);
  if (!lang) {
    process.stderr.write("Usage : npm run i18n:translate -- <langue> [--target=PT-BR] [--force]\n");
    process.exit(2);
  }
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  try {
    const done = await translateCatalog({
      root,
      lang,
      target: target ?? lang.toUpperCase(),
      key: process.env.DEEPL_API_KEY ?? "",
      fetch: globalThis.fetch,
      sleep: (ms) => wait(ms),
      force: args.includes("--force"),
      log: (line) => process.stdout.write(`${line}\n`),
    });
    process.stdout.write(
      `${done.folder} : ${done.parts} parties, ${done.texts} textes. ${done.todos} fonctions gardées en français (TODO), ${done.fallbacks} textes gardés en français (balises perdues). Relis tout avant d'ajouter la langue (docs/LANGUES.md).\n`,
    );
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) await main();
