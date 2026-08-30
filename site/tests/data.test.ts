// Checks the site against content/langbank.json, which is exported from the
// crate by scripts/data-manifest.sh. The reference pages are rendered from the
// manifest on every build, so they cannot drift from it; what these tests
// guard is everything around that — the manifest's own coherence, the
// renderer still writing what the pages promise, and hand-written prose
// staying out of the counting business.
import { describe, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

type Manifest = {
  schema: string;
  version: string;
  counts: {
    languages: number;
    ecosystems: number;
    toolchains: number;
    registries: number;
    tools: number;
    gaps: number;
  };
  facets: { name: string; purpose: string; languages: number }[];
  languages: { id: string; name: string; knows: string[] }[];
  ecosystems: { id: string; languages: string[]; registry: string | null }[];
  toolchains: { id: string; languages: string[] }[];
  registries: { id: string }[];
  tools: { id: string; languages: string[] }[];
  gaps: { subject: string; reason: string }[];
};

const ROOT = join(import.meta.dir, "..");
const CONTENT = join(ROOT, "content");
const REFERENCE = join(CONTENT, "reference");

const manifest: Manifest = JSON.parse(
  readFileSync(join(CONTENT, "langbank.json"), "utf8"),
);

/** The pages tools/render-data.ts writes; everything else is hand-written. */
const RENDERED = new Set([
  "facets.md",
  "languages.md",
  "ecosystems.md",
  "toolchains.md",
  "package-registries.md",
  "tools.md",
  "gaps.md",
]);

describe("data manifest", () => {
  test("is the schema these tests understand", () => {
    expect(manifest.schema).toBe("langbank.data/1");
    expect(manifest.languages.length).toBeGreaterThan(0);
  });

  test("the counts say what the registries hold", () => {
    expect(manifest.counts.languages).toBe(manifest.languages.length);
    expect(manifest.counts.ecosystems).toBe(manifest.ecosystems.length);
    expect(manifest.counts.toolchains).toBe(manifest.toolchains.length);
    expect(manifest.counts.registries).toBe(manifest.registries.length);
    expect(manifest.counts.tools).toBe(manifest.tools.length);
    expect(manifest.counts.gaps).toBe(manifest.gaps.length);
  });

  test("every language another registry names exists", () => {
    const names = new Set(manifest.languages.map((language) => language.name));
    const missing: string[] = [];
    const claim = (owner: string, said: string[]) => {
      for (const name of said)
        if (!names.has(name)) missing.push(`${owner} names ${name}`);
    };
    for (const ecosystem of manifest.ecosystems)
      claim(`ecosystem ${ecosystem.id}`, ecosystem.languages);
    for (const toolchain of manifest.toolchains)
      claim(`toolchain ${toolchain.id}`, toolchain.languages);
    for (const tool of manifest.tools) claim(`tool ${tool.id}`, tool.languages);
    expect(missing).toEqual([]);
  });

  test("every registry an ecosystem resolves against exists", () => {
    const ids = new Set(manifest.registries.map((registry) => registry.id));
    for (const ecosystem of manifest.ecosystems) {
      if (ecosystem.registry !== null)
        expect(ids).toContain(ecosystem.registry);
    }
  });

  test("a facet's coverage never exceeds the languages there are", () => {
    for (const facet of manifest.facets) {
      expect(facet.languages).toBeLessThanOrEqual(manifest.counts.languages);
    }
  });
});

// The renderer runs before bun test (see package.json), so a rendered page
// that is missing or has lost its count means the renderer broke, not that
// the test ran too early.
describe("the rendered pages carry the data", () => {
  const carries: [string, string][] = [
    ["languages.md", `${manifest.counts.languages} languages`],
    ["ecosystems.md", `${manifest.counts.ecosystems} ecosystems`],
    ["toolchains.md", `${manifest.counts.toolchains} toolchains`],
    [
      "package-registries.md",
      `${manifest.counts.registries} package registries`,
    ],
    ["tools.md", `${manifest.counts.tools} tool profiles`],
    ["gaps.md", `${manifest.counts.gaps} recorded absences`],
    ["facets.md", "| `detection` |"],
  ];
  for (const [file, said] of carries) {
    test(`${file} says "${said}"`, () => {
      const where = join(REFERENCE, file);
      expect(existsSync(where)).toBe(true);
      expect(readFileSync(where, "utf8")).toContain(said);
    });
  }

  test("languages.md lists every language", () => {
    const text = readFileSync(join(REFERENCE, "languages.md"), "utf8");
    const rows = text.split("\n").filter((line) => line.startsWith("| "));
    // The header and its underline are the two non-data rows.
    expect(rows.length - 2).toBe(manifest.counts.languages);
  });
});

// Registry sizes move with every data change, so they live only on the pages
// rendered from the manifest. A hand-written page that quotes one is quoting
// a number with an expiry date.
describe("no hand-written page counts a registry", () => {
  const counting =
    /\b\d[\d,]* (languages|ecosystems|toolchains|registries|tools|tool profiles|gaps|recorded absences)\b/;

  function markdownFiles(dir: string): string[] {
    const out: string[] = [];
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) out.push(...markdownFiles(full));
      else if (entry.name.endsWith(".md")) out.push(full);
    }
    return out;
  }

  for (const file of markdownFiles(CONTENT)) {
    const rel = file.slice(CONTENT.length + 1);
    if (RENDERED.has(rel.split("/").pop() ?? "")) continue;
    test(`${rel} quotes no registry count`, () => {
      const hit = counting.exec(readFileSync(file, "utf8"));
      expect(hit?.[0]).toBeUndefined();
    });
  }
});
