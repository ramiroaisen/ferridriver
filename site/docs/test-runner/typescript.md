# TypeScript tests

```bash
npm install -D @ferridriver/test
# or
bun add -d @ferridriver/test
```

## Writing tests

```ts
// tests/login.spec.ts
import { test, expect } from '@ferridriver/test';

test('login flow', async ({ page }) => {
  await page.goto('https://app.example.org/login');
  await page.locator('#email').fill('user@example.org');
  await page.locator('button[type=submit]').click();
  await expect(page).toHaveURL('https://app.example.org/dashboard');
});

test.describe('navigation', () => {
  test('back and forward', async ({ page }) => {
    await page.goto('https://example.org');
    await page.goto('https://example.org/about');
    await page.goBack();
    await expect(page).toHaveURL('https://example.org/');
  });
});
```

## Running

```bash
# All .spec.ts / .test.ts / .feature files in the current tree
npx @ferridriver/test test

# Specific path
npx @ferridriver/test test tests/login.spec.ts

# Common flags
npx @ferridriver/test test -j 4 --retries 1 --headed
npx @ferridriver/test test --reporter junit --output reports/
npx @ferridriver/test test -g smoke --last-failed
```

## CLI subcommands

- `test` — E2E + BDD (mixed)
- `ct` — component tests (see [Component testing](/component-testing/overview))
- `codegen URL` — record interactions, emit Rust / TypeScript / Gherkin
- `install [BROWSER]` — download Chromium

See [CLI reference](/cli/ferridriver-test) for the full flag list.
