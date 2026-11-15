# Initial Prompt

This file preserves the original project request. Markdown headings, lists,
links, tables, and inline code have been added; the wording is unchanged apart
from minor typographic spacing.

> **Editorial note (2026-11-15):** the count-in was later corrected from three
> to four dance beats ("four potatoes"). The original wording below is preserved
> as history; the current rule is in §17 and `AGENTS.md`.

> The plan is the following:

Write a Rust-based contra dance caller training program. Like a karaoke machine displays lyrics to a singer to help them sing the song, this program displays contra dance move order timed to music to help a caller learn to call moves with the correct timing. The background music played with dances can vary, but it should be an ABC music source converted to MIDI.

## The functions

- **Select dance:** Select a dance sourced from contradb.com. An example dance URL. <https://contradb.com/dances/2409>.
- **Select Tune:** Select a tune sourced from abcsessiontunes.com. An example tune URL. <https://abcsessiontunes.com/tunes/75815>
- **Preferences:** Set preferences for the program, including dance move aliases, role term preference(default to `lark/robin`), instrument preferences(default to `fiddle` and `piano`)
- **Begin training session:** Start playing the music and the dance in sync. The music loop is preceded with three beats from the accompaniment based on their first chord played to set the tempo for dancers and the caller. Default to **240 BPM** tempo. Dance moves that the caller must call out are highlighted as time passes, mush like a karaoke machine. Optional setting allows text to speech to relay the moves instead of the user.

> **Critical element:** Dance moves should be called by the caller BEFORE dancers have to execute that move, roughly 1-2 beats depending on how long it takes to call the move. Time the delay based on how long it would take to relay the words, using syllable count as a guide.

Dance moves are written in an abbreviated form, and callers usually make their calls more or less verbose than what appears on the card. Add options to add/modify aliases to specific move types. These aliases will appear in place of the move names when in a training session.

## The user interface

The main interface has most of the bottom space dedicated to the dance information. Near the top of the screen is series of horizontally aligned tabs, labeled **“Select dance”** **“Select Tune”**, and **“Preferences”**. Below these tabs are settings directly pertinent to the practice session, including tempo and playback buttons. This interface should be equally usable on desktop and mobile. Use a grey color scheme, with buttons all being shades of grey on a dark background.

## The music player

The ABC tunes sourced for training are just a fakebook melody that guides a larger band. A common arrangement for contra dance bands is a fiddle/violin melody and a piano or guitar backing track. Copy this arrangement, allowing one to pick melody and backing track instruments. All tunes played should combine a melody part that plays the melody shown in the ABC tune along with an accompaniment based on chords in the tune.

## Select dance

This interface pops up when someone clicks the **“Select dance”** button. It acts as a search engine for contradb.com, where all dances on the site can be selected. Default to **“Another last dance”**. Copy the search options on contradb.com.

## Select Tune

This interface pops up when the user clicks the **“Select tune”** button. It acts as a search engine for abcsessiontunes.com, where all tunes on the site can be selected to be played. Default tune is **“Sandy Boys”**. Copy the search options on abcsessiontunes.com.

## Preferences

List options in this order:

1. **Role preference** (Default `lark/robin`). Research other common contra dance role terms. Replace role terms used in dance cards with the term set by the user.
2. **Dance move aliases:** Button that switches to different interface, where each contra dance move that might appear in a dance has an alias that appears in instructions. Add the ability to override any default aliases set by the program.
3. **Instrument preferences:**
   - **Melody:** Default fiddle. Other options accordion.
   - **Accompaniment:** Default Piano. Other options guitar and accordion.
4. **Application theme:** Dark theme or light theme

## Architecture decisions

- Plan around users eventually being able to source dances and tunes from other websites than the ones I’ve called out.
- Separate logic for parsing dances from the rest of the code so that this logic could be replaced with different logic for parsing dances from another website.
- Cache dances and tunes downloaded so that they don’t need to repeatedly downloaded from the websites fetched from.

## License

This shall be **MIT-licensed**. Feel free to import other MIT-licensed libraries as appropriate for this project. Write a `CREDITS.md` file crediting libraries used.

## Release

Release for the following platforms and install formats:

| Platform | Install format |
| --- | --- |
| Windows | Installer and Portable formats |
| Linux | AppImage, Flatpack and deb |
| Web | Webassembly |
| Android | `.apk` |

## Github actions

Automatically produce new nightly releases after every commit to main. Add a changelog to help track changes. Until I say this is a released project, we are on version `0.1.0`.
