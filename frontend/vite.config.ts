import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwind from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwind()],
  server: {
    host: '127.0.0.3',
    strictPort: true,
    port: 8192,
    proxy: Object.fromEntries(['/api', '/health', '/version', '/openapi.json', '/integration', '/branding'].map(path => [path, { target: process.env.AIHUB_DEV_API_ORIGIN ?? 'http://127.0.0.3:8191' }])),
  },
  test: { environment: 'jsdom', include: ['src/**/*.test.{ts,tsx}'] },
})
