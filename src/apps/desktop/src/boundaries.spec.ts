// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve, sep } from "node:path";

const SELF_FILE_NAME = "boundaries.spec.ts";
const SOURCE_EXTENSIONS = [".ts", ".svelte"];
const PUBLIC_FACE_FILES = ["index", "index.ts", "index.js"];
// `$shared` is the only alias in tsconfig.json and vite.config.ts. Another `$`
// alias would point past the folder rules below.
const ALLOWED_ALIAS = /^\$shared(\/|$)/;
const IMPORT_SPECIFIER =
  /(?:from\s+|import\s*\(\s*|import\s+)["']([^"']+)["']/g;

function listSourceFiles(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return listSourceFiles(path);
    const isSource = SOURCE_EXTENSIONS.some((extension) =>
      entry.name.endsWith(extension),
    );
    return isSource && entry.name !== SELF_FILE_NAME ? [path] : [];
  });
}

function importSpecifiers(file: string): string[] {
  const text = readFileSync(file, "utf8");
  return [...text.matchAll(IMPORT_SPECIFIER)].map((match) => match[1]);
}

function isForbiddenAlias(specifier: string): boolean {
  return specifier.startsWith("$") && !ALLOWED_ALIAS.test(specifier);
}

function segmentsUnder(sourceRoot: string, path: string): string[] {
  return relative(sourceRoot, path).split(sep);
}

function findCycle(edges: Map<string, Set<string>>): string[] | undefined {
  const visiting: string[] = [];
  const finished = new Set<string>();
  const visit = (module: string): string[] | undefined => {
    if (visiting.includes(module))
      return [...visiting.slice(visiting.indexOf(module)), module];
    if (finished.has(module)) return undefined;
    visiting.push(module);
    for (const next of edges.get(module) ?? []) {
      const cycle = visit(next);
      if (cycle) return cycle;
    }
    visiting.pop();
    finished.add(module);
    return undefined;
  };
  for (const module of edges.keys()) {
    const cycle = visit(module);
    if (cycle) return cycle;
  }
  return undefined;
}

export function findViolations(sourceRoot: string): string[] {
  const violations: string[] = [];
  const moduleEdges = new Map<string, Set<string>>();

  for (const file of listSourceFiles(sourceRoot)) {
    const [area, owner] = segmentsUnder(sourceRoot, file);
    for (const specifier of importSpecifiers(file)) {
      if (isForbiddenAlias(specifier)) {
        violations.push(
          `${relative(sourceRoot, file)} imports ${specifier}: only the $shared alias is allowed`,
        );
        continue;
      }
      if (!specifier.startsWith(".")) continue;
      const [targetArea, targetModule, ...rest] = segmentsUnder(
        sourceRoot,
        resolve(dirname(file), specifier),
      );
      const where = `${relative(sourceRoot, file)} imports ${specifier}`;
      if (
        area === "shared" &&
        (targetArea === "core" || targetArea === "modules")
      ) {
        violations.push(`${where}: shared/ must not import core/ or modules/`);
      }
      if (area === "core" && targetArea === "modules") {
        violations.push(`${where}: core/ must not import modules/`);
      }
      if (
        area !== "modules" ||
        targetArea !== "modules" ||
        targetModule === owner
      )
        continue;
      const isPublicFace =
        rest.length === 0 ||
        (rest.length === 1 && PUBLIC_FACE_FILES.includes(rest[0]));
      if (!isPublicFace) {
        violations.push(
          `${where}: deep import, use modules/${targetModule}/index.ts`,
        );
      }
      moduleEdges.set(
        owner,
        (moduleEdges.get(owner) ?? new Set()).add(targetModule),
      );
    }
  }

  const cycle = findCycle(moduleEdges);
  if (cycle)
    violations.push(`import cycle between modules: ${cycle.join(" -> ")}`);
  return violations;
}

describe("webview import boundaries", () => {
  test("the source tree has no violations", () => {
    expect(findViolations(import.meta.dir)).toEqual([]);
  });

  describe("detection", () => {
    function withTree(
      files: Record<string, string>,
      check: (root: string) => void,
    ): void {
      const root = mkdtempSync(join(tmpdir(), "boundaries-"));
      try {
        for (const [path, text] of Object.entries(files)) {
          mkdirSync(dirname(join(root, path)), { recursive: true });
          writeFileSync(join(root, path), text);
        }
        check(root);
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    }

    test("accepts an import through another module's index", () => {
      withTree(
        {
          "modules/a/x.ts": 'import "../b/index.ts";',
          "modules/b/index.ts": "",
        },
        (root) => expect(findViolations(root)).toEqual([]),
      );
    });

    test("rejects a deep import into another module", () => {
      withTree(
        {
          "modules/a/x.ts": 'import "../b/components/y";',
          "modules/b/components/y.ts": "",
        },
        (root) => expect(findViolations(root)).toHaveLength(1),
      );
    });

    test("rejects a cycle between modules", () => {
      withTree(
        {
          "modules/a/index.ts": 'import "../b";',
          "modules/b/index.ts": 'import "../a";',
        },
        (root) => expect(findViolations(root).join()).toContain("import cycle"),
      );
    });

    test("rejects shared importing core", () => {
      withTree(
        { "shared/x.ts": 'import "../core/y";', "core/y.ts": "" },
        (root) => expect(findViolations(root)).toHaveLength(1),
      );
    });

    test("rejects shared importing modules", () => {
      withTree(
        {
          "shared/x.ts": 'import "../modules/a/index";',
          "modules/a/index.ts": "",
        },
        (root) =>
          expect(findViolations(root).join()).toContain(
            "shared/ must not import core/ or modules/",
          ),
      );
    });

    test("rejects core importing modules", () => {
      withTree(
        {
          "core/x.ts": 'import "../modules/a/index";',
          "modules/a/index.ts": "",
        },
        (root) =>
          expect(findViolations(root).join()).toContain(
            "core/ must not import modules/",
          ),
      );
    });

    test("accepts the $shared alias from every area", () => {
      withTree(
        {
          "modules/a/x.ts": 'import "$shared/ui/y";',
          "core/x.ts": 'import "$shared";',
          "shared/ui/y.ts": "",
        },
        (root) => expect(findViolations(root)).toEqual([]),
      );
    });

    test("rejects any alias other than $shared", () => {
      withTree(
        {
          "modules/a/x.ts": 'import "$lib/y";',
          "core/x.ts": 'import("$sharedish/z");',
        },
        (root) => expect(findViolations(root)).toHaveLength(2),
      );
    });
  });
});
