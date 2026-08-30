// Renders content/langbank.json into the reference pages the site serves.
//
// The manifest is exported from the crate by scripts/data-manifest.sh, and
// these pages are written from it on every build rather than committed, so
// they cannot drift from the data: the one copy that can go stale is the
// manifest itself, and CI diffs that against the binary.
//
// The tables mirror what langbank.dev's server renders from the same tables,
// column for column, so a reader moving between the two meets the same shape.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

type Spelling = { requirement: string; case_sensitive: boolean };

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
  languages: {
    id: string;
    name: string;
    role: string;
    extensions: string[];
    primary_extensions: string[];
    knows: string[];
  }[];
  ecosystems: {
    id: string;
    name: string;
    languages: string[];
    manifest: string | null;
    lockfiles: string[];
    registry: string | null;
  }[];
  toolchains: {
    id: string;
    name: string;
    roles: string[];
    languages: string[];
    programs: string[];
    version_probe: string | null;
  }[];
  registries: {
    id: string;
    name: string;
    namespace: Spelling;
    package_name: Spelling;
    version: Spelling;
    repository: string | null;
  }[];
  tools: {
    id: string;
    programs: string[];
    languages: string[];
    configuration: string[];
  }[];
  gaps: { subject: string; facet: string; reason: string; note: string }[];
};

const ROOT = join(import.meta.dir, "..");
const CONTENT = join(ROOT, "content");
const OUT = join(CONTENT, "reference");

const manifest: Manifest = JSON.parse(
  readFileSync(join(CONTENT, "langbank.json"), "utf8"),
);
if (manifest.schema !== "langbank.data/1")
  throw new Error(`unknown manifest schema ${manifest.schema}`);

/** A table cell must stay on one line and keep its pipes to itself. */
const cell = (text: string) => text.replace(/\|/g, "\\|").replace(/\n/g, " ");

const code = (text: string) => `\`${cell(text)}\``;

const codes = (values: string[]) => values.map(code).join(", ");

/** An em dash rather than an empty cell: "nothing here" and "nothing to say"
 * look the same in a table otherwise. */
const orDash = (text: string | null | undefined) => (text ? text : "—");

/** The first six, then how many more — a linter that names eighty languages
 * is a fact, not a row anyone can read. */
function truncated(names: string[]): string {
  const rest = names.length - 6;
  const shown = names.slice(0, 6).map(cell).join(", ");
  return rest > 0 ? `${shown} +${rest}` : orDash(shown);
}

function table(headers: string[], rows: string[][]): string {
  const line = (cells: string[]) => `| ${cells.join(" | ")} |`;
  return [
    line(headers),
    line(headers.map(() => "---")),
    ...rows.map(line),
  ].join("\n");
}

function page(
  file: string,
  front: { title: string; description: string; order: number },
  body: string,
): void {
  const text = [
    "---",
    `title: ${front.title}`,
    `description: ${front.description}`,
    `order: ${front.order}`,
    "---",
    "",
    "<!-- Rendered from langbank.json by tools/render-data.ts; edit that",
    "     script or the data, never this file. -->",
    "",
    body,
    "",
  ].join("\n");
  writeFileSync(join(OUT, file), text);
}

function facets(): void {
  const rows = manifest.facets.map((facet) => [
    code(facet.name),
    cell(facet.purpose),
    String(facet.languages),
  ]);
  page(
    "facets.md",
    {
      title: "Facets",
      description:
        "The eight things langbank can know about a language, and how many languages carry each.",
      order: 2,
    },
    [
      "Eight things langbank can know about a language. Most languages carry",
      "one. A facet is here because a consumer asked for it, and each row says",
      "what carrying it lets that consumer do.",
      "",
      table(["facet", "lets a consumer", "languages"], rows),
    ].join("\n"),
  );
}

/** The web index summarises knowledge the same way, in the cells' hover text. */
function knows(carried: string[]): string {
  if (carried.length === 0) return "nothing but a name";
  return `${carried.length} of 8: ${carried.join(", ")}`;
}

function languages(): void {
  const rows = manifest.languages.map((language) => {
    const hint = language.primary_extensions[0] ?? language.extensions[0];
    return [
      cell(language.name) + (hint ? ` ${code(`.${hint}`)}` : ""),
      cell(language.role),
      knows(language.knows),
    ];
  });
  page(
    "languages.md",
    {
      title: "Languages",
      description:
        "Every language langbank carries, its role, and which of the eight facets it knows.",
      order: 3,
    },
    [
      `${manifest.counts.languages} languages. A language is here because a`,
      "source named it, not because it seemed important; the last column is the",
      "difference between an entry langbank has a paragraph about and one it",
      "has a filename for.",
      "",
      table(["language", "role", "knows"], rows),
    ].join("\n"),
  );
}

function ecosystems(): void {
  const rows = manifest.ecosystems.map((ecosystem) => [
    cell(ecosystem.name),
    truncated(ecosystem.languages),
    orDash(ecosystem.manifest && code(ecosystem.manifest)),
    orDash(codes(ecosystem.lockfiles)),
    orDash(ecosystem.registry && code(`pkg:${ecosystem.registry}`)),
  ]);
  page(
    "ecosystems.md",
    {
      title: "Ecosystems",
      description:
        "Every packaging ecosystem: its manifest, the lockfiles that pin it, and the registry it resolves against.",
      order: 4,
    },
    [
      `${manifest.counts.ecosystems} ecosystems. A manifest, the lockfiles`,
      "that pin it, and the registry it resolves against — what tells a walker",
      "that a directory is a project rather than a folder of files.",
      "",
      table(
        ["ecosystem", "languages", "manifest", "lockfiles", "registry"],
        rows,
      ),
    ].join("\n"),
  );
}

function toolchains(): void {
  const rows = manifest.toolchains.map((toolchain) => [
    cell(toolchain.name),
    cell(toolchain.roles.join(", ")),
    truncated(toolchain.languages),
    codes(toolchain.programs),
    orDash(toolchain.version_probe && code(toolchain.version_probe)),
  ]);
  page(
    "toolchains.md",
    {
      title: "Toolchains",
      description:
        "Every toolchain: what builds, tests, formats and lints each language, and the command that asks its version.",
      order: 5,
    },
    [
      `${manifest.counts.toolchains} toolchains. What builds, tests, formats`,
      "and lints each language, the programs that invoke it, and the command",
      "that asks it its version. The probe is data — langbank states it and the",
      "caller runs it.",
      "",
      table(
        ["toolchain", "kind", "languages", "programs", "version probe"],
        rows,
      ),
    ].join("\n"),
  );
}

/** How identity is spelled, in the words the server uses for the same cell. */
function spelling(component: Spelling): string {
  const folding = component.case_sensitive ? "case-sensitive" : "case-folded";
  return `${component.requirement}, ${folding}`;
}

function registries(): void {
  const rows = manifest.registries.map((registry) => [
    code(`pkg:${registry.id}`),
    cell(registry.name),
    spelling(registry.namespace),
    spelling(registry.package_name),
    orDash(registry.repository && code(registry.repository)),
  ]);
  page(
    "package-registries.md",
    {
      title: "Package registries",
      description:
        "How package identity is spelled in each registry: namespaces, case folding, and default hosts.",
      order: 6,
    },
    [
      `${manifest.counts.registries} package registries. How identity is`,
      "spelled in each: whether a namespace is required, what case-folds, and",
      "where it resolves by default. Two names that differ only in case are the",
      "same package in some registries and not in others.",
      "",
      table(
        ["purl type", "registry", "namespace", "name", "default repository"],
        rows,
      ),
    ].join("\n"),
  );
}

function tools(): void {
  const rows = manifest.tools.map((tool) => [
    code(tool.id),
    codes(tool.programs),
    truncated(tool.languages),
    orDash(codes(tool.configuration)),
  ]);
  page(
    "tools.md",
    {
      title: "Tool profiles",
      description:
        "The programs a repository invokes and the files that configure them.",
      order: 7,
    },
    [
      `${manifest.counts.tools} tool profiles. The programs a repository`,
      "actually invokes, and the files that configure them — enough to",
      "recognise a tool in a CI log or a lockfile without hard-coding its name",
      "in the consumer.",
      "",
      table(["tool", "programs", "languages", "configuration"], rows),
    ].join("\n"),
  );
}

function gaps(): void {
  const rows = manifest.gaps.map((gap) => [
    code(gap.subject),
    code(gap.facet),
    cell(gap.reason),
    cell(gap.note),
  ]);
  page(
    "gaps.md",
    {
      title: "Gaps",
      description:
        "The questions langbank was asked and declined to answer, each with the reason it declined.",
      order: 8,
    },
    [
      `${manifest.counts.gaps} recorded absences. A registry that silently`,
      "omits what it does not know cannot be told apart from one nobody has",
      "filled in; these are the things langbank was asked and declined to",
      "answer. Sources disagreed; only one source said it; it was excluded on",
      "purpose; or it is not modelled yet. Only the last is a to-do.",
      "",
      table(["subject", "facet", "reason", "note"], rows),
    ].join("\n"),
  );
}

facets();
languages();
ecosystems();
toolchains();
registries();
tools();
gaps();
console.log(
  `render-data: wrote 7 reference pages from langbank.json ` +
    `(${manifest.counts.languages} languages, ${manifest.counts.toolchains} toolchains)`,
);
