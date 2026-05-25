# frontend/AGENTS.md

Frontend-specific guidance for agents working under `frontend/`.
The repository-level `../AGENTS.md` still applies.

## Build-Source Tests

`build-src/` scripts keep focused Node tests in the same file as the implementation.

- Use Node's built-in `node:test` and `node:assert/strict`; do not add a frontend test framework for build-source tests.
- Keep test imports static and at the top of the file. Do not use dynamic imports or ad-hoc assertion wrapper types for these tests.
- Keep the file layout in this order:
  - test imports
  - the script spec comment
  - main imports
  - main implementation
  - same-file tests
- Guard CLI execution with Node's test context so `node --test` does not run the script body:

```ts
const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (import.meta.main && !inTest) {
  main();
}

if (inTest) {
  test('...', () => {
    // test body
  });
}
```

- Prefer Promise chains in `build-src/`; do not introduce `async`/`await`.
- Test behavior that documents durable build rules and failure modes. Avoid exhaustive tests for tiny helpers when a higher-level function already covers the behavior.
- Keep all test definitions and test-only helper functions inside the `if (inTest)` block.
- Do not split shared test logic into separate modules. Reusing helper functions inside the same file is fine.
- File-system tests should use repo-local temp directories under `build-test-temp/<script-name>/`. Keep that path ignored in `.gitignore` and included in `pnpm run clean`.
- Keep the `build-test` package script in sync with every `build-src/` file that contains same-file tests.

## Validation

- Run build-source tests with `nix develop --command -- pnpm --dir frontend run build-test`.
- Run `nix develop --command -- pnpm --dir frontend run typecheck` after changing TypeScript.
- Run `nix develop --command -- pnpm --dir frontend run build` when script behavior affects generated assets or HTML.
