// Dump the reference parser's AST for every program in a corpus, as JSON.
//
// Usage: bun reference.ts <path-to-openui-checkout> <corpus.txt> [out.json]
//
// Development aid only: it imports the MIT-licensed reference implementation
// from a local checkout of https://github.com/thesysdev/openui. Nothing from
// that checkout is copied into this repository.

const [checkout, corpus, out] = process.argv.slice(2);
const base = `${checkout}/packages/lang-core/src/parser`;
const { tokenize } = await import(`${base}/lexer.ts`);
const { split, autoClose } = await import(`${base}/statements.ts`);
const { parseExpression } = await import(`${base}/expressions.ts`);

const text = (await Bun.file(corpus).text()).replaceAll("\r\n", "\n");
const programs = text.split("\n-----\n").map((source: string) => {
  const { text: closed } = autoClose(source);
  return split(tokenize(closed)).map((s: { id: string; tokens: unknown[] }) => ({
    id: s.id,
    expr: parseExpression(s.tokens),
  }));
});
// Writing the file directly keeps the bytes UTF-8 whatever the console uses.
if (out) await Bun.write(out, JSON.stringify(programs));
else console.log(JSON.stringify(programs));
