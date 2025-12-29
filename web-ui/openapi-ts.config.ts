import { defineConfig } from "@hey-api/openapi-ts";

export default defineConfig({
  input: {
    path: process.env.OPENAPI_SPEC_PATH || "../openapi.json",
  },
  output: {
    path: "./src/generated",
    format: "prettier",
    lint: "biome",
  },
  plugins: [
    {
      name: "@hey-api/typescript",
      enums: "javascript",
      style: "PascalCase",
    },
    {
      name: "@hey-api/sdk",
      asClass: false,
      operationId: true,
    },
    "@hey-api/client-fetch",
  ],
});
