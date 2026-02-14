import { defineConfig } from 'astro/config';

import tailwind from '@astrojs/tailwind';

// https://astro.build/config
export default defineConfig({
  output: 'static',

  // User should update this
  site: 'https://github.com/user/repo',

  integrations: [tailwind()]
});