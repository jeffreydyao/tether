import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");

  return {
    plugins: [react(), tailwindcss()],

    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },

    server: {
      host: true,
      port: 5173,
      strictPort: true,

      proxy: {
        "/api": {
          target: env.VITE_API_URL || "http://localhost:3000",
          changeOrigin: true,
          secure: false,
        },
      },
    },

    preview: {
      host: true,
      port: 4173,
    },

    build: {
      outDir: "dist",
      sourcemap: false,
      minify: "esbuild",

      rollupOptions: {
        output: {
          manualChunks: {
            vendor: ["react", "react-dom"],
            ui: [
              "@radix-ui/react-dialog",
              "@radix-ui/react-tabs",
              "@radix-ui/react-select",
              "@radix-ui/react-switch",
              "vaul",
            ],
          },
        },
      },

      target: ["es2022", "chrome111", "firefox128", "safari16.4"],
      chunkSizeWarningLimit: 600,
    },

    envPrefix: "VITE_",
  };
});
