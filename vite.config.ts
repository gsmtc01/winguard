import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: ["es2021", "chrome105", "safari14"],
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./vitest.setup.ts",
    coverage: {
      provider: "v8",
      // 커버리지는 앱 소스만 센다. website/ 의 정적 사이트 스크립트는 테스트
      // 대상이 아니면서 분모만 키워, CI 의 커버리지 하한선(래칫)을 엉뚱하게
      // 끌어내린다.
      include: ["src/**"],
    },
  },
});
