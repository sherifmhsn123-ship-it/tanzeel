import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const cssPath = resolve(process.cwd(), 'src/styles.css');

const forbidden = [
  { name: 'physical left/right property', pattern: /(^|[\s;{])(left|right)\s*:/m },
  { name: 'physical block inline property', pattern: /(padding|margin|border)-(left|right)\b/m },
  { name: 'physical text alignment', pattern: /text-align\s*:\s*(left|right)\b/m },
  { name: 'physical float', pattern: /float\s*:\s*(left|right)\b/m },
];

describe('layout styles', () => {
  it('uses logical CSS properties so RTL and LTR share one stylesheet', () => {
    const css = readFileSync(cssPath, 'utf8');
    const violations = forbidden
      .filter((rule) => rule.pattern.test(css))
      .map((rule) => rule.name);
    expect(violations).toEqual([]);
  });
});
