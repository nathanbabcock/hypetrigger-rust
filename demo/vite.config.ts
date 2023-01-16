import { defineConfig, searchForWorkspaceRoot } from 'vite'
import tsconfigPaths from 'vite-tsconfig-paths'
import solidPlugin from 'vite-plugin-solid'

export default defineConfig({
  plugins: [tsconfigPaths(), solidPlugin()],
  build: {
    target: 'esnext',
  },
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
