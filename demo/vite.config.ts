import { defineConfig, searchForWorkspaceRoot } from 'vite'
import tsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
  plugins: [tsconfigPaths()],
  // build: {
  //   target: 'esnext',
  //   polyfillDynamicImport: false,
  // },
  server: {
    open: true,
    fs: {
      allow: [
        // search up for workspace root
        searchForWorkspaceRoot(process.cwd()),
        // your custom rules
        '../lib-rust'
      ]
    }
  }
})
