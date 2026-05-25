# frontend/AGENTS.md

Frontend-specific guidance for agents maintaining code under `frontend/`.
The repository-level `../AGENTS.md` still applies.

## File Layout

- Keep frontend code flat within one-level directories; prefer patterns like `src/*.ts`, `src/*.comp.ts`, `src/*.pure.ts`, `build-src/*.ts`, and `tests/*.test.ts`.
- Use suffixes to describe maintenance intent and to keep tooling such as UnoCSS source scanning aligned with the code shape:
  - `*.comp.ts`: DOM/UI component code.
  - `*.pure.ts`: code intentionally kept valid in both browser and Node runtimes; avoid browser-only globals, DOM types, and `crypto` globals here.
  - plain `*.ts`: frontend code that is not committed to the component or pure-runtime conventions.

## TypeScript Style

- Use Promise APIs over `await` syntax where practical. The project owner dislikes `await`; this is a preference, not a deep technical rule.
- Use Node's built-in `node:test` and `node:assert/strict`; do not add a frontend test framework.
- Do not add browser UI test tooling such as Playwright; UI-level testing is intentionally out of scope because it is too heavy for this project.
- Test only selected behavior that can be implemented as pure modules.
- Keep test imports static and at the top of the file. Do not use dynamic imports or ad-hoc assertion wrapper types.
- Do not create modules only for tests. Repetition in test files is acceptable because test-only shared modules can make specs affect each other indirectly.
- Reusing helper functions inside the same test file is fine.

## Build-Source Tests

`build-src/` scripts run in Node, so their implementation and tests stay in the same file.

- Keep `build-src/*.ts` files in this order:
  - test imports
  - spec comment
  - main imports
  - main implementation
  - test implementation
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

- Keep all test definitions and test-only helper functions inside the `if (inTest)` block.
- File-system tests should use repo-local temp directories under `build-test-temp/<script-name>/`. Keep that path ignored in `.gitignore` and included in `pnpm run clean`.
- Keep the `build-test` package script in sync with every `build-src/` file that contains same-file tests.

## Pure Runtime Tests

`src/*.pure.ts` modules can be tested from Node, but they are still frontend runtime modules.

- The `.pure.ts` suffix marks intentional runtime portability only; it does not mean every pure module must have tests.
- Keep selected pure tests in `tests/*.test.ts`, because `src` implementation and Node tests run in different runtime contexts.
- Put durable behavioral specs in JSDoc on the tested function, value, or type.
- Do not duplicate specs in test code; tests should contain test implementation only.

## Validation

- Run build-source tests with `nix develop --command -- pnpm --dir frontend run build-test`.
- Run pure runtime module tests with `nix develop --command -- pnpm --dir frontend run test`.
- Run `nix develop --command -- pnpm --dir frontend run typecheck` after changing TypeScript.
- Run `nix develop --command -- pnpm --dir frontend run build` when script behavior affects generated assets or HTML.
