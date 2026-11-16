---
title: Role terms
description: Choose Larks/Robins, Larks/Ravens, or another role vocabulary, or enter custom singular and plural terms, and see the card update.
sidebar_position: 4
---

ContraDB cards use role words such as `ladles` and `gentlespoons`. Callers use
many vocabularies. Contra Karaoke replaces recognised role words in both the
dance card and the rendered calls.

## Open the setting

<Steps>
  <Step shot="prefs-open">
    Select **Preferences** in the top tab row.
  </Step>
  <Step shot="prefs-roles-presets">
    Find the **Role terms** section. The current preset is selected; the italic
    example below it updates as you change terms.
  </Step>
</Steps>

## Choose a preset

The presets are:

- **Larks / Robins** — the default.
- **Larks / Ravens**
- **Ladies / Gentlemen**
- **Women / Men**
- **Leads / Follows**
- **Lefts / Rights**
- **Custom**

Every label names the left-side term first, then the right-side term
(`Larks / Robins`), so the list reads the same way even when a dance card writes
its pair in the other order.

Selecting a named preset replaces all four role fields: left singular, left
plural, right singular, and right plural. **Custom** keeps the current words so
you can edit them.

## Enter custom terms

<Steps>
  <Step shot="prefs-roles-custom">
    Choose **Custom**, then edit **Left singular**, **Left plural**, **Right
    singular**, and **Right plural**.
  </Step>
</Steps>

Typing in any of the four fields keeps **Custom** selected; the preset does not
snap back to a named one, even if the words happen to match it. The example line
updates as you type. The dance card and calls are rebuilt when you leave the
field, so a long word does not rebuild the timeline on every keystroke. The
selection and terms are saved, so they are still there after a restart.

Use the plural forms for collective phrases (`larks allemande`) and the singular
forms for individual phrases (`swing your lark`). The example line shows how
a mixed card phrase is rewritten.

## Where replacement applies

Replacement is word-boundary based and applies to recognised role words,
including:

- ladles / ladies / larks / leads / lefts / women / gals / belles;
- gentlespoons / gentlemen / gents / robins / ravens / follows / rights / men /
  guys / beaus.

It deliberately does **not** replace `shadow`, `neighbor`, or `partner`; those
are not role-side terms. It also does not turn `circle left` into `circle
rights`, because plain `left` and `right` direction words are only replaced when
they appear as plural role terms (`lefts`, `rights`).

## See the result in the session

<Steps>
  <Step shot="session-roles-applied">
    Return to **Session**. The card and the call prompt use the new terms.
  </Step>
</Steps>

Role changes rebuild the call timeline, because the rendered words feed the lead
calculation. If the session had music loaded, it is marked pending; press
**Play** to render again.

## If a term is not replaced

The classifier only knows role words it can recognise. A custom move phrase
that embeds a role word in an unusual form may need a
[custom alias](/docs/guides/aliases) instead of a role preset. Custom aliases are
rendered through the same role replacement, so use the normal role words in the
alias template.

Next: [Dance move aliases](/docs/guides/aliases).
