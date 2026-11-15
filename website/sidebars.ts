import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    'intro',
    {
      type: 'category',
      label: 'Getting started',
      collapsed: false,
      items: [
        'getting-started/install-windows',
        'getting-started/install-linux',
        'getting-started/install-web',
        'getting-started/install-android',
        'getting-started/build-and-run',
        'getting-started/first-session',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      collapsed: false,
      items: [
        'guides/choosing-a-dance',
        'guides/choosing-a-tune',
        'guides/session-controls',
        'guides/role-terms',
        'guides/aliases',
        'guides/instruments-and-soundfonts',
        'guides/timing-and-lead',
        'guides/text-to-speech',
        'guides/cache-and-offline',
        'guides/themes-and-accessibility',
        'guides/export-midi',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: [
        'reference/keyboard-shortcuts',
        'reference/how-calls-are-timed',
        'reference/data-sources',
        'reference/privacy',
        'reference/known-limitations',
        'reference/troubleshooting',
      ],
    },
    {
      type: 'category',
      label: 'Contributing',
      collapsed: true,
      items: ['contributing/docs-and-screenshots'],
    },
  ],
};

export default sidebars;
