import {defineConfig} from 'vite';
import {svelte} from '@sveltejs/vite-plugin-svelte';

// Apeireth 桌面伙伴 — Vite 6 (从 Pattern 移植; 端口 1420 与 Tauri devUrl 对齐)
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'chrome105',
    sourcemap: false,
    rollupOptions: {
      output: {
        manualChunks: {
          // 重量级第三方拆独立 vendor chunk (katex/hljs 是 markdown 渲染大头)
          'vendor-markdown': ['marked', 'dompurify', 'highlight.js', 'katex'],
          'vendor-svelte': ['svelte'],
          'vendor-lucide': ['lucide-svelte'],
        },
      },
    },
  },
});
