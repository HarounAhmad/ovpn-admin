import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig({
  base: '/ui/',
  plugins: [svelte()],
  build: { outDir: '../webui-dist', emptyOutDir: true },
  server: { port: 5173, strictPort: true }
})

