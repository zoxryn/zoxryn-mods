import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  workers: 2,
  reporter: [['list'], ['json', { outputFile: 'verification/interface-tests.json' }]],
  use: {
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 1320, height: 880 },
    browserName: 'chromium',
    channel: process.env.ZOXRYN_TEST_BROWSER,
  },
  webServer: {
    command: 'node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 1420',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: true,
  },
});
