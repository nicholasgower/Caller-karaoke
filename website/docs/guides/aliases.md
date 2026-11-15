---
title: Dance move aliases
description: Override the words shown and spoken for a move type, use placeholders, reset defaults, and add aliases for unmatched figures.
sidebar_position: 5
---

The dance card is written in shorthand. Callers expand it differently: one
caller says `swing your neighbor`, another says `neighbor swing`. Aliases let you
change the rendered call without changing the source card.

The alias is not just display text. The lead time is calculated from the
rendered words, so a longer alias is automatically moved earlier.

## Open the alias editor

<Steps>
  <Step shot="prefs-open">
    Select **Preferences**.
  </Step>
  <Step shot="prefs-alias-list">
    Select **Edit dance move aliases…** under **Role terms**.
  </Step>
</Steps>

The screen lists every recognised move type. An entry marked **overridden** has
a custom template. The **Filter** field narrows the list by move name or id.

## Edit one move type

<Steps>
  <Step shot="prefs-alias-editor">
    Find a move such as **Swing**, edit its template, and leave the field. The
    change rebuilds the call timeline.
  </Step>
</Steps>

The defaults are examples, not rules. A few from the current table:

| Move type | Default template | Example call |
| --- | --- | --- |
| Swing | `swing your {actor_singular}` | `swing your neighbor` |
| Do-si-do | `{actor} do-si-do {count}` | `neighbors do-si-do once` |
| Allemande | `{actor} allemande {direction} {count}` | `larks allemande left once and a half` |
| Balance and swing | `{actor} balance and swing` | `partners balance and swing` |
| Circle | `circle {direction} {count}` | `circle left three places` |
| Long lines | `long lines forward and back` | `long lines forward and back` |

## Placeholders

<Shot id="prefs-alias-placeholders" />

The editor supports five placeholders:

| Placeholder | Filled with |
| --- | --- |
| `{actor}` | The actor as written on the card, using your role terms (`neighbors`, `larks`, `partners`). |
| `{actor_singular}` | A singular form (`neighbor`, `lark`, `partner`). |
| `{actor_object}` | A possessive object form (`your neighbor`, `your partner`). |
| `{direction}` | The direction parsed from the card (`left`, `right`, `across`). |
| `{count}` | The count parsed from the card (`once`, `twice`, `three places`, `1½`). |

Unknown placeholders are left as literal text, so a typo is visible in the
session rather than silently disappearing. Empty placeholders are removed and
extra spaces are cleaned up.

## Reset

<Steps>
  <Step shot="prefs-alias-reset">
    Select **Reset to default** on one row to restore that move's built-in
    template.
  </Step>
  <Step shot="prefs-alias-reset-all">
    Select **Reset all aliases** to restore every move template **and remove all
    custom figure aliases**.
  </Step>
</Steps>

Reset all is broader than it sounds: it clears the override table and the custom
alias table in one action.

## Add an alias for an unmatched figure

The classifier recognises many common moves, but not every card phrase. When a
figure is classified as **Other**, a custom alias can supply its wording.

<Steps>
  <Step shot="prefs-alias-custom">
    Under **Custom figure aliases**, enter the card phrase and the words to say,
    then select **Add alias**.
  </Step>
  <Step shot="session-alias-call">
    Return to the session and check the rendered call. Remove the custom alias
    with **Remove** if it is wrong.
  </Step>
</Steps>

The phrase is matched after normalisation (case, punctuation, and spacing), so
`Strange thing!` and `strange thing` use the same entry.

A custom alias only applies when the classifier does not already recognise the
phrase. To change a recognised move such as Swing, edit the move-type template
instead.

## Sub-calls and splitting

A figure row normally produces one call. Two separators create sub-calls:

- a semicolon (`;`);
- a spaced dash (` - `, ` – `, or ` — `).

`balance & swing` stays one call because `&` is not a separator. If a row is
split, the row's beats are divided between the sub-calls; a trailing balance
gets about two beats when they fit.

## What triggers a rebuild

Any alias edit rebuilds the call timeline and sets the music to pending. The
next **Play** renders the new calls and their lead times. The role preset and
custom role fields rebuild in the same way.

Next: [Instruments and soundfonts](/docs/guides/instruments-and-soundfonts).
