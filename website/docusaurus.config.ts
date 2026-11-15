import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Contra Karaoke',
  tagline: 'Learn to call contra dances in time with the music',
  favicon: 'img/favicon.svg',
  url: 'https://storytold.github.io',
  baseUrl: '/Contra-karaoke/',
  organizationName: 'storytold',
  projectName: 'Contra-karaoke',
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
            'https://github.com/storytold/Contra-karaoke/edit/main/website/',
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
      title: 'Contra Karaoke',
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
          href: 'https://github.com/storytold/Contra-karaoke',
          label: 'GitHub',
          position: 'right',
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
            {label: 'GitHub', href: 'https://github.com/storytold/Contra-karaoke'},
            {label: 'Roadmap', href: 'https://github.com/storytold/Contra-karaoke/blob/main/ROADMAP.md'},
            {label: 'License', href: 'https://github.com/storytold/Contra-karaoke/blob/main/LICENSE'},
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Contra Karaoke. MIT licensed.`,
    },
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 3,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
