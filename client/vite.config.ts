import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import path from "path";

export default defineConfig({
    plugins: [react(), nodePolyfills()],
    server: { cors: true, allowedHosts: ['cuda.network'] },
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src"),
            'vm-browserify': path.resolve(__dirname, './empty.js'),
        },
    },
    optimizeDeps: {
        exclude: ['vm-browserify'],
    },
    build: {
        chunkSizeWarningLimit: 1600,
        rollupOptions: {
            input: {
                main: path.resolve(__dirname, 'index.html')
            },
            output: {
                entryFileNames: `assets/[name].js`,
                chunkFileNames: `assets/[name].chunk.js`,
                assetFileNames: `assets/[name].[ext]`,
                manualChunks: {
                    react: ["react", "react-dom", "react-redux", "react-router-dom"],
                    libraries: ["antd", "@reduxjs/toolkit"]
                },
            },
            external: ['vm-browserify'],
            onwarn(warning, warn) {
                if (warning.code === "MODULE_LEVEL_DIRECTIVE" || warning.code === 'EVAL') {
                    return;
                }

                warn(warning);
            },
        },
    },
});