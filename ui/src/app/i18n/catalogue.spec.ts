import en from './en.json';
import fr from './fr.json';

interface Node {
  readonly [key: string]: string | Node;
}

const flatten = (node: Node, prefix = ''): string[] =>
  Object.entries(node).flatMap(([key, value]) =>
    typeof value === 'string' ? [`${prefix}${key}`] : flatten(value, `${prefix}${key}.`),
  );

const placeholders = (text: string): string[] => [...text.matchAll(/\{\{\s*(\w+)\s*\}\}/g)].map((match) => match[1]!).sort();

const lookup = (node: Node, key: string): string =>
  key.split('.').reduce<string | Node>((current, part) => (typeof current === 'string' ? current : (current[part] as string | Node)), node) as string;

describe('translation catalogues', () => {
  const englishKeys = flatten(en as Node);
  const frenchKeys = flatten(fr as Node);

  it('defines exactly the same keys in English and French', () => {
    expect(frenchKeys.filter((key) => !englishKeys.includes(key))).toEqual([]);
    expect(englishKeys.filter((key) => !frenchKeys.includes(key))).toEqual([]);
  });

  it('keeps the same interpolation parameters in both languages', () => {
    const mismatches = englishKeys.filter(
      (key) => placeholders(lookup(en as Node, key)).join() !== placeholders(lookup(fr as Node, key)).join(),
    );
    expect(mismatches).toEqual([]);
  });

  it('has no empty translation', () => {
    expect([...englishKeys.filter((key) => lookup(en as Node, key) === ''), ...frenchKeys.filter((key) => lookup(fr as Node, key) === '')]).toEqual([]);
  });
});
