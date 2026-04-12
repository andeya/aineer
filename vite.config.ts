import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;
const browserTarget =
  process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari15";

/**
 * Patch lookbehind assertions that older WebKit (Safari < 16.4) cannot parse.
 * mdast-util-gfm-autolink-literal v2 uses `(?<=^|\s|\p{P}|\p{S})` which
 * throws "SyntaxError: Invalid regular expression: invalid group specifier
 * name" on macOS 12 Tauri WebView.
 *
 * Neither esbuild, oxc, nor Rolldown can transpile regex syntax, so we do a
 * string-level replacement during Vite's dep optimization and source transform.
 *
 * See: https://github.com/syntax-tree/mdast-util-gfm-autolink-literal/issues/10
 */
function patchLookbehindPlugin(): Plugin {
  return {
    name: "patch-lookbehind",
    enforce: "pre",
    transform(code, id) {
      if (
        !id.includes("mdast-util-gfm-autolink-literal") &&
        !id.includes("remark-gfm")
      ) {
        return null;
      }
      if (!code.includes("(?<=")) {
        return null;
      }
      return {
        code: code.replace(
          "(?<=^|\\s|\\p{P}|\\p{S})",
          "(?:^|[\\s\\p{P}\\p{S}])",
        ),
        map: null,
      };
    },
  };
}

/**
 * Rolldown-compatible plugin for patching lookbehinds during the
 * optimizeDeps pre-bundling phase (Vite 8 uses Rolldown).
 */
function patchLookbehindRolldownPlugin() {
  return {
    name: "patch-lookbehind-rolldown",
    load(id: string) {
      if (!id.includes("mdast-util-gfm-autolink-literal")) {
        return null;
      }
      let code = readFileSync(id, "utf-8");
      if (code.includes("(?<=")) {
        code = code.replace(
          "(?<=^|\\s|\\p{P}|\\p{S})",
          "(?:^|[\\s\\p{P}\\p{S}])",
        );
      }
      return { code, map: null };
    },
  };
}

export default defineConfig({
  plugins: [patchLookbehindPlugin(), react(), tailwindcss()],

  clearScreen: false,

  resolve: {
    alias: {
      "@": resolve(__dirname, "ui"),
    },
  },

  oxc: {
    target: browserTarget,
  },
  optimizeDeps: {
    rolldownOptions: {
      plugins: [patchLookbehindRolldownPlugin()],
    },
  },

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      ignored: ["**/app/**"],
    },
  },

  envPrefix: ["VITE_", "TAURI_ENV_*"],

  build: {
    target: browserTarget,
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    outDir: "dist",
  },
});
