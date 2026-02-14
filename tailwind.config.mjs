/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{astro,html,js,jsx,md,mdx,sss,ts,tsx,vue}'],
	theme: {
		extend: {
			colors: {
				cyber: {
					bg: '#0f172a',
					card: '#1e293b',
					cyan: '#06b6d4',
					pink: '#ec4899',
					green: '#10b981',
				}
			},
			fontFamily: {
				sans: ['Inter', 'sans-serif'],
				impact: ['Bebas Neue', 'sans-serif'],
				mono: ['JetBrains Mono', 'monospace'],
			},
		},
	},
	plugins: [],
};
