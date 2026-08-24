import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

const apiTarget = process.env.MMP_API_TARGET ?? 'http://localhost:7979';

const proxy = {
  target: apiTarget,
  changeOrigin: true,
};

export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    port: 5173,
    proxy: {
      '/api': proxy,
      '/openapi.json': proxy,
      '/docs': proxy,
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
  },
});
