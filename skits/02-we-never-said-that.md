# Skit 2 — "We Never Said That"

**Runtime** ~40s · **Format** 9:16 vertical · **Cast** 2

The joke: the tool works, and then immediately refuses to take credit for more
than it did. The punchline is the honesty.

No real person is named or depicted. The characters are invented.

---

## Shot list

Tags: **[LIVE]** shoot it · **[AI]** generated cutaway, no people · **[SCREEN]** screen recording

| # | Time | Tag | Shot | Action / Dialogue | On-screen |
|---|---|---|---|---|---|
| 1 | 0:00–0:04 | [LIVE] | JO at laptop, lit by screen | **JO:** "They took it down. Now they're saying it never said that." | — |
| 2 | 0:04–0:06 | [LIVE] | SAM, mid-snack, uninvested | **SAM:** "Screenshot it?" | — |
| 3 | 0:06–0:10 | [LIVE] | JO | **JO:** "Screenshots aren't proof. Anyone can fake a screenshot." | — |
| 4 | 0:10–0:12 | [LIVE] | SAM, still chewing | **SAM:** "Did you seal it?" | — |
| 5 | 0:12–0:14 | [LIVE] | JO | **JO:** "...Did I *what?*" | — |
| 6 | 0:14–0:19 | [SCREEN] | Terminal, full bleed | SAM reaches over and types. | `$ provenance verify ./evidence`<br>`intact: content matches the sealed manifest` |
| 7 | 0:19–0:22 | [LIVE] | JO, sitting up | **JO:** "So that proves they published it." | — |
| 8 | 0:22–0:27 | [LIVE] | SAM, flat | **SAM:** "No. It proves your copy hasn't changed since you sealed it." | — |
| 9 | 0:27–0:30 | [LIVE] | JO, deflating | **JO:** "...That's significantly less impressive." | — |
| 10 | 0:30–0:35 | [LIVE] | SAM, shrug | **SAM:** "It's the part that's actually true. The rest is your job." | — |
| 11 | 0:35–0:38 | [AI] | Cutaway | Laptop screen in a dark room, terminal text scrolling, cursor blinking. | — |
| 12 | 0:38–0:41 | [EITHER] | Card | — | **PROVENANCE**<br>Proves the bytes. The sourcing is on you. |

---

## Script (clean read)

> **JO:** They took it down. Now they're saying it never said that.
> **SAM:** Screenshot it?
> **JO:** Screenshots aren't proof. Anyone can fake a screenshot.
> **SAM:** Did you seal it?
> **JO:** ...Did I *what?*
> *(SAM types. Terminal: `intact: content matches the sealed manifest`)*
> **JO:** So that proves they published it.
> **SAM:** No. It proves your copy hasn't changed since you sealed it.
> **JO:** ...That's significantly less impressive.
> **SAM:** It's the part that's actually true. The rest is your job.

---

## Direction notes

- **SAM never gets excited.** Not when it works, not when correcting JO. The
  flatness is the performance.
- Shot 8 is the real line. Don't rush it and don't play it as a gotcha — SAM is
  being precise, not clever. A tool that oversells itself is the thing this
  whole bit is making fun of.
- JO's deflation in shot 9 should be genuine disappointment. They wanted a
  smoking gun and got a fact.
- Shot 6: record the terminal for real. `cargo build --release` then run it —
  the actual output is in the repo README. Fake terminal text reads fake.

## Alternate ending (swap shot 12)

> **JO:** So what do I do?
> **SAM:** Call the person who sent it to you and ask them a question you already
> know the answer to.
> **JO:** That's just journalism.
> **SAM:** *(mouth full)* Yeah.

## Coverage to grab

- SAM's "Did you seal it?" ×3 — deadpan, bored, and one with a full mouth.
- JO's reaction to shot 8, clean single.
- Real screen recording of `provenance seal` *and* `provenance verify`, plus one
  showing a `TAMPERED` result — you'll want the red version for other cuts.
