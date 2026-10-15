// How an answer reads in the island: models write a little Markdown (paragraphs, lists,
// **bold**, `code`, fenced code). This turns it into plain pieces that the chat draws as
// text nodes: nothing of an answer ever becomes HTML.

export type Span = { text: string; bold?: boolean; code?: boolean };

export type Block =
  | { kind: "p"; spans: Span[] }
  | { kind: "item"; mark: string; spans: Span[] }
  | { kind: "code"; text: string }
  | { kind: "title"; spans: Span[] };

/** `**bold**` and `` `code` `` within a line; anything unclosed stays as typed. */
export function spans(line: string): Span[] {
  const out: Span[] = [];
  const pattern = /\*\*([^*]+)\*\*|`([^`]+)`/g;
  let last = 0;
  for (const match of line.matchAll(pattern)) {
    const at = match.index ?? 0;
    if (at > last) out.push({ text: line.slice(last, at) });
    if (match[1] !== undefined) out.push({ text: match[1], bold: true });
    else out.push({ text: match[2], code: true });
    last = at + match[0].length;
  }
  if (last < line.length) out.push({ text: line.slice(last) });
  return out;
}

/** The blocks of an answer. An unfinished code fence (still streaming) shows as code. */
export function blocks(text: string): Block[] {
  const out: Block[] = [];
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  let paragraph: string[] = [];
  let code: string[] | null = null;
  const flush = (): void => {
    if (paragraph.length) out.push({ kind: "p", spans: spans(paragraph.join(" ")) });
    paragraph = [];
  };
  for (const line of lines) {
    if (code) {
      if (/^\s*```/.test(line)) {
        out.push({ kind: "code", text: code.join("\n") });
        code = null;
      } else {
        code.push(line);
      }
      continue;
    }
    if (/^\s*```/.test(line)) {
      flush();
      code = [];
      continue;
    }
    const item = /^\s*(?:([-*•])|(\d+)[.)])\s+(.*)$/.exec(line);
    const title = /^\s*#{1,6}\s+(.*)$/.exec(line);
    if (item) {
      flush();
      out.push({ kind: "item", mark: item[2] ? `${item[2]}.` : "•", spans: spans(item[3]) });
    } else if (title) {
      flush();
      out.push({ kind: "title", spans: spans(title[1]) });
    } else if (line.trim() === "") {
      flush();
    } else {
      paragraph.push(line.trim());
    }
  }
  flush();
  if (code) out.push({ kind: "code", text: code.join("\n") });
  return out;
}
