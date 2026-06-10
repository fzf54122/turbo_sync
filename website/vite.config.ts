import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const pagesBase = process.env.VITE_PAGES_BASE ?? '/';

export default defineConfig({
  base: pagesBase,
  plugins: [react()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 5173,
  },
});
