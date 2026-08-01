# Skits

Two short vertical skits about a document nobody checked. Written to be shot in
one room with a phone, or assembled from AI cutaways, or mixed.

| | Skit | Runtime | Cast |
|---|---|---|---|
| 1 | [A Source](01-a-source.md) | ~35s | 3 (2 works) |
| 2 | [We Never Said That](02-we-never-said-that.md) | ~40s | 2 |

**Every character is invented.** These are about a *situation* — a document
circulating faster than anyone can verify it — not about any real person. No real
individual is named, depicted, or impersonated, and the AI cutaways contain no
faces at all. Keep it that way if you write more: the comedy is in the newsroom,
not in the likeness, and it is funnier for it.

## The shot tags

Each shot in the scripts is tagged, so you can take any of three routes:

| Tag | Meaning |
|---|---|
| **[LIVE]** | You shoot it. Dialogue, faces, timing. |
| **[AI]** | Generated cutaway, no people. Drop in as-is. |
| **[SCREEN]** | Screen recording of the real CLI. |
| **[EITHER]** | Title card — motion graphic or a piece of paper on camera. |

**Shoot it all yourself:** ignore the tags, shoot every row, use the AI clips as
B-roll or bin them.

**Assemble without a crew:** shoot only the `[LIVE]` dialogue on a phone; the
`[AI]` and `[SCREEN]` shots carry the rest.

**Mix:** the default. AI clips open and close each skit, you carry the middle.

## The generated cutaways

Made with Higgsfield (Kling 3.0 Turbo), 1080p, 9:16, 5s each — no people, no
faces, so they cut into anything and raise no likeness questions.

| Used in | Content | Job |
|---|---|---|
| Skit 1, shot 1 | Overhead: folder slams onto a cluttered desk, papers scatter | `7fad3aa6-6536-4280-b406-f0bb8c511a7c` |
| Skit 2, shot 11 | Close-up: terminal text scrolling on a laptop in a dark room | `d1de71a0-7fa5-443f-b0b9-14886d34e14b` |

The clips are **not committed** — they live in Higgsfield and were generated
from a sandbox whose egress policy blocks the CDN, so they could not be pulled
in here. Fetch them yourself into `media/`:

```bash
./fetch-media.sh
```

Both are silent. Add room tone or they'll feel dead next to your live audio.

## Record the terminal for real

Skit 2 shot 6 shows actual output. Fake terminal text reads fake — use the real
thing:

```bash
cargo build --release
mkdir -p /tmp/evidence && echo "page one" > /tmp/evidence/a.txt
./target/release/provenance seal /tmp/evidence --note "copy as received"
./target/release/provenance verify /tmp/evidence
```

For a `TAMPERED` take, edit the file between seal and verify. Grab both — the
red one is useful for other cuts.

## Cutting it together

Shot files named `01-03.mp4` (skit-shot) sort correctly, which makes the rest
trivial.

**Concatenate a whole skit** — re-encodes, so mismatched sources still join:

```bash
for f in shots/01-*.mp4; do echo "file '$PWD/$f'"; done > /tmp/list.txt
ffmpeg -f concat -safe 0 -i /tmp/list.txt \
  -vf "scale=1080:1920:force_original_aspect_ratio=decrease,\
pad=1080:1920:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30" \
  -c:v libx264 -crf 18 -c:a aac -ar 48000 skit-01.mp4
```

**Trim one take** (`-ss` before `-i` seeks fast; `-c copy` avoids re-encoding but
snaps to the nearest keyframe — drop it if you need frame accuracy):

```bash
ffmpeg -ss 00:00:03.5 -to 00:00:06.2 -i raw-take.mp4 -c copy shots/01-04.mp4
```

**Give a silent AI cutaway an audio track**, so concat doesn't drop the stream:

```bash
ffmpeg -i media/cutaway-folder-slam.mp4 -f lavfi -i anullsrc=r=48000:cl=stereo \
  -shortest -c:v copy -c:a aac shots/01-01.mp4
```

**Burn in a title card:**

```bash
ffmpeg -i skit-01.mp4 -vf "drawtext=text='seal it before you share it':\
fontsize=52:fontcolor=white:x=(w-tw)/2:y=h-260:enable='gte(t,32)'" \
  -c:a copy skit-01-tagged.mp4
```

Mind the mismatch when mixing: the AI clips are 1080×1920 @ 30fps and silent;
phone footage is often 60fps with audio. The concat filter above normalises
resolution and frame rate, and the anullsrc trick fixes the missing audio track.
Skip either and ffmpeg will either refuse or silently drop your sound.

## If you want more

The scripts are deliberately over-covered — each has an alternate ending and a
coverage list. Two skits at ~35s cut down to four 15s posts without reshooting.
