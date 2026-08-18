export type Inline =
  | { kind: 'text'; text: string }
  | { kind: 'strong'; text: string }
  | { kind: 'em'; text: string }
  | { kind: 'code'; text: string }
  | { kind: 'link'; text: string; href: string };

export type Block =
  | { kind: 'heading'; level: 1 | 2 | 3; content: Inline[] }
  | { kind: 'paragraph'; content: Inline[] }
  | { kind: 'list'; ordered: boolean; items: Inline[][] }
  | { kind: 'code'; text: string }
  | { kind: 'rule' };

const INLINE = /(\*\*[^*]+\*\*|__[^_]+__|\*[^*]+\*|_[^_]+_|`[^`]+`|\[[^\]]+\]\([^)]+\))/;

function parseInline(raw: string): Inline[] {
  const out: Inline[] = [];
  let rest = raw;

  while (rest.length) {
    const match = INLINE.exec(rest);
    if (!match || match.index === undefined) {
      out.push({ kind: 'text', text: rest });
      break;
    }
    if (match.index > 0) out.push({ kind: 'text', text: rest.slice(0, match.index) });

    const token = match[0];
    if (token.startsWith('**') || token.startsWith('__')) {
      out.push({ kind: 'strong', text: token.slice(2, -2) });
    } else if (token.startsWith('`')) {
      out.push({ kind: 'code', text: token.slice(1, -1) });
    } else if (token.startsWith('[')) {
      const split = token.indexOf('](');
      const text = token.slice(1, split);
      const href = token.slice(split + 2, -1).trim();
      if (href.startsWith('https://')) out.push({ kind: 'link', text, href });
      else out.push({ kind: 'text', text });
    } else {
      out.push({ kind: 'em', text: token.slice(1, -1) });
    }
    rest = rest.slice(match.index + token.length);
  }

  return out.filter((i) => i.kind !== 'text' || i.text.length > 0);
}

export function parseMarkdown(source: string): Block[] {
  const blocks: Block[] = [];
  const lines = source.replace(/\r\n/g, '\n').split('\n');

  let i = 0;
  let paragraph: string[] = [];

  const flush = () => {
    if (paragraph.length) {
      blocks.push({ kind: 'paragraph', content: parseInline(paragraph.join(' ')) });
      paragraph = [];
    }
  };

  while (i < lines.length) {
    const line = lines[i];

    if (line.trim() === '') {
      flush();
      i++;
      continue;
    }

    if (/^```/.test(line)) {
      flush();
      const body: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) body.push(lines[i++]);
      i++;
      blocks.push({ kind: 'code', text: body.join('\n') });
      continue;
    }

    if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      flush();
      blocks.push({ kind: 'rule' });
      i++;
      continue;
    }

    const heading = /^(#{1,3})\s+(.*)$/.exec(line);
    if (heading) {
      flush();
      blocks.push({
        kind: 'heading',
        level: heading[1].length as 1 | 2 | 3,
        content: parseInline(heading[2]),
      });
      i++;
      continue;
    }

    const bullet = /^\s*[-*+]\s+(.*)$/.exec(line);
    const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
    if (bullet || numbered) {
      flush();
      const ordered = Boolean(numbered);
      const items: Inline[][] = [];
      while (i < lines.length) {
        const b = /^\s*[-*+]\s+(.*)$/.exec(lines[i]);
        const n = /^\s*\d+[.)]\s+(.*)$/.exec(lines[i]);
        const m = ordered ? n : b;
        if (!m) break;
        items.push(parseInline(m[1]));
        i++;
      }
      blocks.push({ kind: 'list', ordered, items });
      continue;
    }

    paragraph.push(line.trim());
    i++;
  }

  flush();
  return blocks;
}
