import { defineConfig } from '@playwright/test';

const live = process.env.BASE_URL;
const base = live ? (live.endsWith('/') ? live : `${live}/`) : 'http://127.0.0.1:4173/little-quest/';

export default defineConfig({
  testDir: 'tests',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: true,
  workers: process.env.CI ? 2 : 4,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list']],
  use: { baseURL: base, trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  webServer: live ? undefined : { command: 'node server.mjs', url: base, reuseExistingServer: !process.env.CI },
});
