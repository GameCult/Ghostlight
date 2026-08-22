import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "@gamecult/eve-contracts": fileURLToPath(
        new URL("../vendor/eve/packages/eve-contracts/dist/index.js", import.meta.url),
      ),
      "ajv/dist/2020.js": fileURLToPath(
        new URL("./node_modules/ajv/dist/2020.js", import.meta.url),
      ),
    },
  },
});
