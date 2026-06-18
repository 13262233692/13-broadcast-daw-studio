/** @type {import('tailwindcss').Config} */

export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{js,ts,vue}"],
  theme: {
    container: {
      center: true,
    },
    extend: {
      colors: {
        daw: {
          bg: '#0d0d11',
          panel: '#1a1a1e',
          'panel-deep': '#121217',
          green: '#00ff88',
          blue: '#00a8ff',
          solo: '#ffcc00',
          red: '#ff3b3b',
          text: '#e0e0e0',
          muted: '#888888',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
};
