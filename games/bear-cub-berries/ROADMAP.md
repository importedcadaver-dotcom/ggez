# Bear Cub Berries — audit and roadmap

Written after the game was lifted out of `examples/` into this crate. The point of
this document is to be honest about what is good, what is fragile, and what is
worth doing next. Recommendations are ordered by value per hour, not by how fun
they are to build.

## Where it stands

* **Scope:** a complete loop. Title card, intro cutscene, ten days of play, boons,
  a boss, an epilogue, a losing screen, restart.
* **Size:** one file, ~3300 lines, ~40 functions on `impl Game`.
* **State of build:** `cargo build -p bear-cub-berries` is clean, zero warnings,
  `rustfmt` clean.
* **Assets:** ~500 KB total (sprites, nine WAVs, one font). All local, no network.
* **Tests:** none. Everything verified so far was verified by screenshotting a
  running window and posting synthetic key events to it.

## What the audit pass changed

A pass over dialogue, framing, environment and the frame budget. Measured, not
guessed: at 1045x700 in a debug build the game holds 144 fps / ~6.8 ms per frame,
which is the vsync cap, so there is no headroom left to measure on this machine
and no optimising claim is being made here. What was actually broken and is now
fixed:

* **Dialogue had no owner.** Lines collided, stacked on the HUD and never said who
  was speaking. All speech now goes through one arbiter: a speaker name, a priority,
  a drop rule when two bubbles are already up, a stack offset so two lines cannot
  overlap, and hard wrapping. Story lines clear the small talk under them.
* **The treeline grew into the readouts.** Canopies are now clamped below the day
  and clock text, which were previously illegible against dark green.
* **The meadow was three ellipses and a rectangle.** It now has an edge treeline on
  both sides, a fallen trunk with a cut end and bark ridges, rocks, mushrooms, leaf
  litter, a cold seasonal light grade past day five, and snow from day six.
* **The title had no plate.** Menu text sat directly on the den and the cub; it now
  sits on a panel over a darkened meadow, with a best-day readout.
* **Cutscenes were text on a dimmed map.** They now letterbox, label the speaker,
  and show progress dots. Remaining: skip-on-hold and a drawn image per beat.
* **No frame readout.** F1 now prints fps and frame time alongside the rest.

## Architecture: the honest version

The file is organised in sections (tuning, palette, structures, boons, rig,
enemies, game, helpers) and the sections hold up fine. What does not hold up:

1. **One `Game` struct with 60+ fields.** Every system reaches into every other
   system's state. That is why the file works at all — and why every new feature
   costs more than the last one.
2. **Drawing and simulation live in the same `impl`.** `update_day` mutates the
   world; `draw` reads it; both are enormous (200+ lines each) and both are
   touched by nearly every change.
3. **Content is hard-coded in match arms.** Boons are an enum with `name()`,
   `blurb()` and a dozen `self.count(Boon::X)` call sites scattered through the
   logic. Adding an eleventh boon means editing speed, carry, pickup reach, hive
   capacity, regrow time, and the HUD. That is a design smell, not laziness.
4. **Tuning constants are named and grouped** (`DAY_ONE_SECONDS`, `BOSS_DAY`,
   `SWIPE_RANGE`, `CLOSE_BOND`) — this is genuinely good, keep doing it.
5. **No determinism seam.** `seeded_rng()` seeds from the system clock, so no run
   can be reproduced. That blocks regression testing, replay, and balance work.
6. **Audio is optional-by-construction** (`Option<SoundData>` per effect, queued
   and flushed once per frame) — this is the most robust part of the codebase and
   the pattern the rest should copy.

## Repository health

* The game was previously sitting in `examples/` as `06_bear_cubs.rs`, which is
  wrong twice: examples are documentation that upstream rebuilds on every CI run,
  and a 3300-line game is not documentation. It is now a real crate at
  `games/bear-cub-berries` and a workspace member.
* **`Cargo.lock` is gitignored** by the library's `.gitignore`, which is correct
  for a library and wrong for the executable now in the workspace. Anyone
  building the game gets unpinned transitive dependency resolution.
* **No CI for the game.** The library's CI builds examples; nothing builds or
  formats this crate.
* **Asset licensing is thin.** The bear sheets have no written provenance in the
  repo, the sound pack is named but not licensed in-tree, and the font ships with
  its OFL text (good). A `NOTICE`/`ASSETS.md` claiming each file's origin is worth
  thirty minutes.
* **The build is coupled to the library.** A path dependency on `../..` means the
  game cannot be built without the engine source, and the engine's default
  features (`audio`, `gamepad`) are pulled in whole — `gilrs` in particular needs
  `libudev` on Linux.

## Priorities

### P0 — do these before adding another feature

1. **Split `main.rs` into modules.** Minimum viable split:
   `main.rs` (boot), `game/mod.rs` (state + update), `game/day.rs` (the loop),
   `game/foes.rs`, `game/draw.rs`, `game/ui.rs` (title, cutscene, boons, over),
   `content.rs` (boons, chatter, cutscene text), `audio.rs` (the `Sfx` bank).
   Do not redesign anything while moving — pure code motion, verified by building.
2. **Make runs reproducible.** Add `--seed <u64>` (and an env var) and thread one
   `Rand32` through everything. Then a bug reports as "seed 1234, day 3", and a
   balance change can be A/B'd on the same seed.
3. **Add a smoke test that does not need a window.** With the split above, the day
   loop becomes callable with a `dt` and a direction vector; a test can simulate
   ten days headlessly and assert that the quota is reachable and nothing panics.
4. **Commit `Cargo.lock`** for the workspace now that it contains a binary.

### P1 — the things that would make it a better game

5. **Rarity for boons (the Hades part).** Offers currently have no variance. Give
   each offer a rarity that scales its effect (common 1.0, rare 1.5, legendary 2.0),
   colour the card border accordingly, and replace the scattered `count()` calls
   with a single `power(boon) -> f32`. This is also the refactor that fixes smell
   #3 above.
6. **Meta-progression across runs.** Right now a lost run teaches you nothing. Even
   a tiny mirror — "bravery" earned per day survived, spent on start-of-run perks
   saved to `user_data_dir` — turns the loop into a roguelite proper. `ggez`
   already exposes `ctx.filesystem.user_data_dir()`.
7. **A pause menu.** `Escape` currently quits instantly with no confirmation. Pause
   with resume / restart / quit / volume is table stakes.
8. **Controller support.** `gilrs` is already compiled in and completely unused.
9. **Enemy sprite pass.** The bears are pixel art and the badger/boar/bees are
   smooth vector shapes. Either draw pixel enemies to match (best) or deliberately
   restyle the bears' vector fallback rigs and use those everywhere (cheapest).
10. **Balance pass with a real player.** Day 10 asks for 24 food in ~61 seconds.
    That is only possible via honey (worth 2) and hive regrowth. It has never been
    played end to end; treat `WIN_DAY`, `BOSS_DAY`, boss HP and the quota step as
    unvalidated numbers.

### P2 — polish and delight

11. **More comfort actions.** Grooming Pip, carrying him to the sunny patch,
    sleeping in a pile for a day-end buff. The bond system is the most distinctive
    thing here and it currently has two verbs.
12. **Rival den / second area.** A ridge map (the berries are "fatter up there")
    with a longer walk, better food, and worse animals would give the run shape.
13. **Persistent scars.** Injuries already persist within a run; letting a bad day
    leave a visible mark on the cub across runs is cheap and thematically right.
14. **Cutscene polish.** Letterboxing, speaker labels and progress dots are in. A held key to skip,
    a slow text reveal, and a drawn illustration per beat would sell it.
15. **Settings file.** Volume, mute, fullscreen, and whether the intro plays.
16. **Accessibility.** Remappable keys, a high-contrast palette option, and a
    reduced-motion flag for the screen shake and sparkles.

## One structural recommendation

The single highest-leverage change is P0.1 + P0.2 together: split the file and make
runs seedable. Everything else on this list gets cheaper afterwards, because right
now every change requires re-verifying a 3300-line file by posting keystrokes into
a window and squinting at a screenshot.
