import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://eternalmac.dev',
  integrations: [
    starlight({
      title: 'eternalMac',
      logo: {
        src: './public/logo.svg',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/eternalMac/eternalMac',
        },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Overview', slug: 'docs' },
            { label: 'Installation', slug: 'docs/getting-started/installation' },
            { label: 'Server Setup', slug: 'docs/getting-started/server-setup' },
            { label: 'Client Setup', slug: 'docs/getting-started/client-setup' },
          ],
        },
        {
          label: 'Usage',
          items: [
            { label: 'Attach', slug: 'docs/usage/attach' },
            { label: 'Sessions', slug: 'docs/usage/sessions' },
            { label: 'Sync', slug: 'docs/usage/sync' },
          ],
        },
        {
          label: 'Operations',
          items: [
            { label: 'Status and Doctor', slug: 'docs/operations/status-and-doctor' },
            { label: 'Troubleshooting', slug: 'docs/operations/troubleshooting' },
            { label: 'Commands', slug: 'docs/reference/commands' },
          ],
        },
      ],
    }),
  ],
});
