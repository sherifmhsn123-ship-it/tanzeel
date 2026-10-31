import { describe, expect, it } from 'vitest';
import ar from '../i18n/ar.json';
import en from '../i18n/en.json';

type JsonObject = { [key: string]: string | JsonObject };

function flatten(value: JsonObject, prefix = '', out = new Map<string, string>()): Map<string, string> {
  for (const [key, child] of Object.entries(value)) {
    const path = prefix === '' ? key : `${prefix}.${key}`;
    if (typeof child === 'string') {
      out.set(path, child);
    } else {
      flatten(child, path, out);
    }
  }
  return out;
}

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{\{\s*([a-zA-Z0-9_]+)\s*\}\}/g)]
    .map((match) => match[1])
    .sort();
}

const arFlat = flatten(ar as JsonObject);
const enFlat = flatten(en as JsonObject);

describe('i18n resources', () => {
  it('has identical key sets in Arabic and English', () => {
    const arKeys = [...arFlat.keys()].sort();
    const enKeys = [...enFlat.keys()].sort();
    expect(enKeys).toEqual(arKeys);
  });

  it('has no empty translations', () => {
    for (const [key, value] of arFlat) {
      expect(value.trim(), `ar.${key}`).not.toBe('');
    }
    for (const [key, value] of enFlat) {
      expect(value.trim(), `en.${key}`).not.toBe('');
    }
  });

  it('uses the same interpolation placeholders in both languages', () => {
    for (const [key, arValue] of arFlat) {
      const enValue = enFlat.get(key);
      expect(enValue, `en.${key}`).toBeDefined();
      expect(placeholders(enValue as string), `placeholders for ${key}`).toEqual(
        placeholders(arValue),
      );
    }
  });
});
