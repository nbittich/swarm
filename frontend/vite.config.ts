import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
    build: {
        rollupOptions: {
            output: {
                manualChunks: {
                    vendor: ['react', 'react-dom', 'react-redux', 'cron-validate', 'uuid', 'react-router-dom', '@reduxjs/toolkit'],
                    ui: ['@ant-design/colors', 'antd'],
                    yasgui: ['@zazuko/yasgui']
                }
            }
        }
    },
    plugins: [react()],
    resolve: {
        alias: {
            "@swarm": "/src",
        },
    },
    server: {
        proxy: {
            '/api': {
                target: 'http://localhost:8080',
                changeOrigin: true,
                secure: false,
                // rewrite: (path) => path.replace(/^\/api/, ''),
            },
        },
    },
})
