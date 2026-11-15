# Dance-term pronunciation review

Contra Karaoke speaks calls with the local Kokoro-82M voice on desktop. Kokoro
does not read English letter by letter; it consumes phonemes from Misaki's
grapheme-to-phoneme (G2P) model. That model is accurate for most of the contra
vocabulary, but a few terms are not.

This page records the review, the corrections, and the tests that keep them in
place. The default voice is Kokoro `af_heart` (US English, 24 kHz); the other
selectable voices use the same G2P and pronunciation overrides. The corrections
live in `crates/core/src/pronounce.rs`.

## How the review was done

1. Phonemize every default alias, every role term, and every count with the
   same Misaki G2P the app uses.
2. Normalize Misaki's IPA to Kokoro's trained alphabet (`A` = /eɪ/, `I` = /aɪ/,
   `O` = /oʊ/, `W` = /aʊ/, `Y` = /ɔɪ/, `ʤ` and `ʧ` are single tokens).
3. Compare each result with the conventional contra pronunciation.
4. Inject an exact phoneme override for any term that did not match.
5. Re-run the review against the real model with the ignored smoke test (see
   below).

The check is automated by `cargo test -p contra-app`. The default test suite
does not download anything.

## Corrections

| Term | Problem | Kokoro phonemes | Platform-TTS respelling |
| --- | --- | --- | --- |
| `poussette` | G2P produced "pows-ET" (rhymes with "cows") | `puˈsɛt` ("poo-SET") | `poo set` |
| `petronella` | Pinned to the contra stress pattern, with the first syllable stressed | `pˈɛtɹOnˌɛlə` ("PET-roh-nel-uh") | `petronelluh` |
| `promenade` | G2P chose "prom-uh-NAHD" | `pɹˌɑmənˈAd` ("prom-uh-NADE") | unchanged |
| `Rory O'More` | The tokenizer split `O'More` and spelled it out | `ɹˈɔːɹi ˈO mˈɔːɹ` | `Rory O More` |
| `do-si-do` | Requested "doe see doe"; G2P said "doo see doo" | `dˈO sˈiː dˈO` | `doe see doe` |
| `gentlespoon(s)` | G2P did not know the term and spelled it letter by letter | `ˈɡɛntəlspuːn(z)` | `gentle spoon(s)` |
| `gyre` | Correct as `ʤˈIɚ` ("jire"); platform voices vary | unchanged | `jire` |

Notes:

- `petronella` is pinned to the first-syllable contra stress
  ("PET-roh-nel-uh"), so a future G2P or lexicon update cannot silently move
  it.
- `promenade` is the one that most affects a call: US contra callers say
  "prom-uh-NADE", not the French-influenced "prom-uh-NAHD".
- `O'More` is normalized to `O More` before G2P; the G2P tokenizer otherwise
  treats the apostrophe as a word boundary and reads `O`, `'`, and `More`
  separately.

## Terms that were already correct

The review also checked the rest of the default vocabulary. These need no
override:

| Term | Kokoro phonemes |
| --- | --- |
| `larks` | `lˈɑːɹks` |
| `robins` | `ɹˈɑːbɪnz` |
| `neighbors` | `nˈAbɚz` |
| `partners` | `pˈɑːɹtnɚz` |
| `ladies` | `lˈAdiz` |
| `gentlemen` | `ʤˈɛntəlmən` |
| `allemande` | `ˈæləmˌænd` |
| `balance` | `bˈæləns` |
| `star` | `stˈɑːɹ` |
| `circle` | `sˈɜːkəl` |
| `chain` | `ʧˈAn` |
| `swing` | `swˈɪŋ` |
| `gyre` | `ʤˈIɚ` ("jire") |
| `hey` | `hˈA` ("hay") |
| `contra` | `kˈɑntɹə` |
| `see saw` | `sˈiː sˈɔː` |
| `box the gnat` | `bˈɑːks ðə nˈæt` |
| `swat the flea` | `swˈɑːt ðə flˈiː` |
| `mad robin` | `mˈæd ɹˈɑːbɪn` |
| `california twirl` | `kˌæləfˈɔɹnjə twˈɜːl` |
| `butterfly whirl` | `bˈʌɾɚflˌI wˈɜːl` |
| `revolving door` | `ɹᵻvˈɑːlvɪŋ dˈɔːɹ` |
| `meltdown swing` | `mˈɛltdWn swˈɪŋ` |
| `figure eight` | `fˈɪɡjɚ ˈAt` |
| `once and a half` | `wˈʌns ænd ɐ hˈæf` |
| `twice and a half` | `twˈIs ænd ɐ hˈæf` |
| `three places` | `θɹˈiː plˈAsᵻz` |

## Timing check

The neural voice was measured against the syllable-based lead for every default
call at 240 BPM. The calibrated long call fits comfortably:

```
larks allemande left once and a half
  measured 2.22 s   lead 2.75 s   (0.53 s to spare)
```

Short calls can run about 0.1-0.2 s past the minimum lead because the voice has
a natural onset and release; the default 0.5 s safety padding absorbs most of
it. If a stricter gap is wanted, raise the minimum speech lead or the safety
padding in Preferences. The timeline calibration itself is unchanged.

## Running the checks

```sh
# Phoneme review, role terms, counts, and the bundled dance. No network.
cargo test -p contra-app

# Real model inference. Point the cache at a directory containing
# kokoro-v1.0/model_quantized.onnx, af_heart.bin, and config.json.
CONTRA_KOKORO_CACHE=/path/to/cache \
  cargo test -p contra-app synthesizes_a_reviewed_call_from_the_real_model -- --ignored
```

The ignored test loads the ONNX Runtime (embedded on Windows, downloaded on
Linux), loads the model, and synthesizes "larks allemande left once and a half"
plus every corrected term, asserting that each result is non-silent and long
enough to be a phrase.
