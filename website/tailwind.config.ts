import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        display: ['"Noto Serif SC"', '"LXGW WenKai"', 'Georgia', 'serif'],
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"SFMono-Regular"', 'Consolas', 'monospace'],
      },
      colors: {
        midnight: '#07111f',
        panel: '#0e1c31',
        cyanline: '#54d8ff',
        cobalt: '#4d7cff',
      },
    },
  },
  plugins: [],
} satisfies Config;
