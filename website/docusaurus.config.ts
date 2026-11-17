import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Caller Karaoke',
  tagline: 'Learn to call contra dances in time with the music',
  favicon: 'img/favicon.svg',
  url: 'https://callerkaraoke.nicholasgower.com',
  // Keep in sync with the hosted app public URL in scripts/build-web-app.mjs.
  baseUrl: '/',
  organizationName: 'nicholasgower',
  projectName: 'Caller-karaoke',
  trailingSlash: true,
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl:
            'https://github.com/nicholasgower/Caller-karaoke/edit/main/website/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  plugins: [
    [
      '@easyops-cn/docusaurus-search-local',
      {
        hashed: true,
        language: ['en'],
        indexDocs: true,
        docsRouteBasePath: '/docs',
      },
    ],
  ],
  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: 'Caller Karaoke',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          position: 'left',
          label: 'Guide',
        },
        {
          to: '/download',
          label: 'Download',
          position: 'left',
        },
        {
          to: '/play',
          label: 'Play',
          position: 'left',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Guide',
          items: [
            {label: 'Your first session', to: '/docs/getting-started/first-session'},
            {label: 'Choose a dance', to: '/docs/guides/choosing-a-dance'},
            {label: 'Choose a tune', to: '/docs/guides/choosing-a-tune'},
          ],
        },
        {
          title: 'Reference',
          items: [
            {label: 'Keyboard shortcuts', to: '/docs/reference/keyboard-shortcuts'},
            {label: 'How calls are timed', to: '/docs/reference/how-calls-are-timed'},
            {label: 'Data sources', to: '/docs/reference/data-sources'},
            {label: 'Known limitations', to: '/docs/reference/known-limitations'},
            {label: 'Privacy', to: '/docs/reference/privacy'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'GitHub', href: 'https://github.com/nicholasgower/Caller-karaoke'},
            {label: 'Roadmap', href: 'https://github.com/nicholasgower/Caller-karaoke/blob/main/ROADMAP.md'},
            {label: 'License', href: 'https://github.com/nicholasgower/Caller-karaoke/blob/main/LICENSE'},
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Caller Karaoke. MIT licensed.`,
    },
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 3,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
