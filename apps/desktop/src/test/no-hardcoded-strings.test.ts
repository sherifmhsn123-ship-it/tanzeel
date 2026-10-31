import { readFileSync, readdirSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const srcDir = resolve(process.cwd(), 'src');
const excludedDirs = new Set(['test', 'i18n']);

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!excludedDirs.has(entry.name)) {
        collectSourceFiles(join(dir, entry.name), out);
      }
      continue;
    }
    const ext = extname(entry.name);
    if (ext === '.ts' || ext === '.tsx') {
      out.push(join(dir, entry.name));
    }
  }
  return out;
}

const files = collectSourceFiles(srcDir);

describe('hardcoded interface strings', () => {
  it('keeps Arabic text out of components', () => {
    const offenders: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      if (/[\u0600-\u06ff]/u.test(content)) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });

  it('does not render literal text inside JSX elements', () => {
    const offenders: string[] = [];
    const jsxText = /(?<!=)>\s*[A-Za-z][^<>{}]*[A-Za-z]\s*</g;
    for (const file of files) {
      if (!file.endsWith('.tsx')) {
        continue;
      }
      const content = readFileSync(file, 'utf8');
      if (jsxText.test(content)) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });

  it('does not hardcode user-facing attribute text', () => {
    const offenders: string[] = [];
    const attributeText = /\b(aria-label|title|placeholder|alt)\s*=\s*"[^"]*[A-Za-z\u0600-\u06ff][^"]*"/g;
    for (const file of files) {
      if (!file.endsWith('.tsx')) {
        continue;
      }
      const content = readFileSync(file, 'utf8');
      if (attributeText.test(content)) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });
});
