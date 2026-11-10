# Ritual GPUI implementation plan

Develop Ritual inside the complete Quiet.md app. The document workspace is the launch view throughout this plan. Add each Ritual workflow alongside the existing features. Choose individual replacements only when their turn comes. Each milestone ends with a workflow Nick can repeat in the desktop app. The first is **Water → 2 glasses → quit → reopen**. The next three make it possible to capture that event from anywhere, explain its context, and use the evidence in an experiment report.

## Starting point

The Quiet.md development handoff is complete. The checkout at `~/Desktop/ritual-gpui` has the upstream history, branch `codex/ritual-gpui-dev`, and baseline commit `fcd2affb7fb56b3aa9362f2c4c88bd686cc52f05`. The core test run passed 102 tests with two ignored, and a note survived a full desktop restart. The subsequent development Dock icon uses Ritual’s mark and a DEV badge. Keep Rust 1.99.1, the lockfile, and the vendored GPUI, Taffy, and xattr as the starting point. [Pinned upstream tree](https://github.com/s1dashu/quiet-md/tree/fcd2affb7fb56b3aa9362f2c4c88bd686cc52f05)

To run the **upstream reference workspace** on this Mac after editing Rust, quit the development app with ⌘Q and run the following from a terminal.

```sh
cd "$HOME/Desktop/ritual-gpui"
export QUIET_MD_HOME="$HOME/Library/Application Support/Ritual GPUI Dev/quiet-baseline"
export QUIET_MD_LOG_DIR="$HOME/Library/Logs/Ritual GPUI Dev/quiet-baseline"
cargo run --locked --bin quiet_md --features gpui/runtime_shaders
```

The shader flag is the verified build path with this Mac’s Command Line Tools. Use the existing release checkout at `~/Desktop/ritual-release-0.1.1-prep-recovered` for Ritual’s design tokens, contracts, and interaction references.

## 1. Water survives a restart

**Development handoff.** Quit with ⌘Q, then rebuild and open Ritual from a fresh terminal:

```sh
cd "$HOME/Desktop/ritual-gpui"
./scripts/run-ritual-dev.sh
```

The launcher builds the Cargo development binary and runs it in `target/debug/Ritual Dev.app`. The default Mac profile is `~/Library/Application Support/Ritual GPUI Dev/ritual.sqlite3`. Set `RITUAL_GPUI_DATA_DIR` to a fresh directory to try the empty state separately. The focused storage suite runs with `cargo test --locked -p ritual-core`.

**User workflow.** Choose **Home → + → Custom**. Set Title to **Water**, select **Glasses** from Metric, then choose **Start Tracking**. Select Water from Home’s list or cards. Enter **2** and choose **Log**. History shows **2 glasses** and the local time. Quit the process with ⌘Q. Reopen the same profile. The document workspace opens as usual. Choose **Home → Water**.

1. **Extend the original workspace.** Keep the sidebar, document editor and tree, Home, settings, themes, Quick Note, note library, guide seeding, menus, file handling, and shortcuts active. ⌘N creates a document. Make tracked habits the first tab on Home, beside My documents and Favorites. Use the production Ritual index’s centered name/value list, with Quiet’s list/card switch and its own saved layout preference. The upper-right + on Home, in the main editor, and in folders opens a searchable picker for custom habits and presets; a configuration step shows title, metric and today’s start date. Device metrics can be recorded manually; provider authentication and automatic sync are separate milestones. Keep document and folder creation in the document tree and their contextual toolbars. Both launch and Dock reopen use the original workspace constructor. Keep the development note library at `~/Library/Application Support/Ritual GPUI Dev/quiet-baseline` and the habit database at `~/Library/Application Support/Ritual GPUI Dev/ritual.sqlite3`. Honor `QUIET_MD_HOME` and `RITUAL_GPUI_DATA_DIR` for isolated test profiles.
2. **Put writes in a GPUI-independent Rust crate.** Add `crates/ritual-core` to the workspace. Open SQLite at a supplied path, run versioned migrations transactionally, enable foreign keys on each connection, and update the lockfile for the chosen driver. Expose create/list habits and append/list logs. Store stable IDs, trimmed required habit fields, UTC timestamps, positive finite amounts, the local event day, manual source, and completion status. Index history by habit, completion time, and ID.
3. **Make a submission safe to retry.** Allocate a habit ID or log event ID once per submission. Enforce uniqueness on `(habit_id, client_event_id)`. An identical retry returns the committed record. Conflicting data under the same ID returns an error. Run storage off the UI thread, keep failed drafts editable, and apply history results only to the currently selected habit. Opening an unreadable database shows the error and a retry action.

**Pass on desktop.** After relaunch, verify Water’s configured name, unit, and exactly one entry of 2 glasses. Intentionally log **1 glass** and verify two entries, newest first. Test double-submit, retry after a failed write, invalid amounts, unknown habits, and switching selection during a read. Run `cargo test --locked -p ritual-core` and the applicable workspace checks. Record the executable, test directory, relaunch command, and before/after evidence. Use this restart check again at every milestone. Also exercise ⌘N, note editing and persistence, Home, and switching between a document and Habits in the same window.

## 2. Quick capture from another app

**User workflow.** With another Mac app focused, invoke Ritual’s configurable shortcut. Type **2 glasses of water after the run**. The capture window shows Water, 2 glasses, the event time, and the optional note “after the run.” Return commits the entry and returns focus to the previous app. A plain **2 glasses of water** is enough on an ordinary day.

- Adapt the global hotkey and window lifecycle in [Quiet’s Quick Note](../src/quick_note.rs). Open one small Ritual composer and focus it on repeated invocation. Keep Quiet’s global Quick Note binding. The existing Ritual voice HUD also uses ⌥Space. Offer an independently configurable binding for Ritual capture, surface conflicts, and provide an in-app **Quick capture** command. Typing and future voice input should feed the same draft.
- Start with matching an existing habit and its configured unit. Show editable habit, quantity, event time, and note before submission. Ambiguity opens the picker. Escape cancels. Use the Water milestone’s append command and retry identity from both windows. Save the optional note with the log in one transaction, following Ritual’s existing `HabitLogBase.notes` field. Record when the note was entered so backdating a log preserves both times.

**Pass on desktop.** Invoke capture twice and verify one composer. Cancel a draft and check the history count. Submit the example, exercise a failed save and retry, then reopen Ritual. The one new entry has the intended amount, time, and note. Repeat with the shortcut occupied and verify the in-app command still works.

## 3. Linked observations in Logs and Calendar

**User workflow.** Open yesterday’s Water entry from Calendar and add “after the run” if it has no note. In the same day view, add **Overnight flight** as a standalone observation. Link it with `@` to a relevant log. Open that log and follow the backlink to the observation. Search for **flight** a week later and return to the original day.

- Give observations a stable ID, text, an event day or time window, and a separate authored-at timestamp. A note entered the next morning belongs to the day the user selects. Migrate each nonempty quick-capture note into one observation linked to its original log. Preserve the text, log ID, and recorded event time. Resolve the legacy note field from that observation wherever compatibility requires it. Subsequent edits update one copy.
- Use Ritual’s existing `EntityRef` convention of type plus ID. Begin with habits, logs, days, and time windows. Add experiments when their screen lands. Adapt Quiet’s keyboard mention picker, result snippets, and backlink rows. Persist links with the observation and index both directions. Renames should update display labels. A removed target should show as unavailable while the authored text stays readable. [Quiet mentions](../src/editor/links.rs) · [Quiet search](../src/search.rs)
- Assemble the day from committed logs and observations. Clicking a history item opens its day or record. Reuse this navigation from report results in Milestone 4. Search the indexed text with day, habit, and source filters. A day with only measurements remains complete and usable. Observation text contributes context and never increments a metric.

**Pass on desktop.** Upgrade a Milestone 2 profile, restart twice, and find the original note exactly once. Create a day-only observation without logging any habit. Link it to a log, rename the habit, and verify navigation in both directions after relaunch. Remove the link and confirm the backlink disappears. Search for **flight**, open the matching day, and verify Water’s total is identical before and after these edits.

## 4. Experiment reports with traceable results

**User workflow.** Create **Afternoon caffeine and sleep**. Record the question, the comparison period, and the sleep measure before reviewing results. Add the flight observation as evidence. Compare nights following afternoon caffeine with nights explicitly confirmed caffeine-free for that interval. Open a result to inspect its nights, source records, and observations. Write a conclusion and save a dated report.

1. **Use Ritual’s experiment and report models.** Extend the existing concepts of observations, files, metrics, and conclusions, with typed links back to records. Offer a short template with the question, what changed, outcome measure, dates, evidence, conclusion, and what to try next. Use Quiet’s paragraphs, lists, paste, undo, and draft autosave where people write. Preserve drafts across failed saves. Keep source measurements in Ritual’s store. [Quiet editor architecture](ARCHITECTURE.md)
2. **Implement one inspectable comparison.** Start with manually entered caffeine and sleep records or a labeled test fixture. Specify the local cutoff, reporting time zone, and which subsequent sleep session belongs to each day. Show mean sleep duration, units, eligible night count per group, and exclusions. A **No afternoon caffeine** action records an explicit confirmation for that day. Keep this separate from free-text context and from the positive-amount log command. An unlogged day is unknown. A missing sleep duration contributes no hours or nights to the mean.
3. **Separate live review from a saved conclusion.** A live result recomputes after a correction. Saving a report revision captures the question, date range, time zone, comparison definition, values, counts, exclusions, source IDs and the values used, linked observation excerpts, author, and save time. Reopen or export that revision as Markdown with the evidence and local references. A later revision can incorporate corrected records while the earlier conclusion keeps its original evidence.

**Pass on desktop.** Use two qualifying caffeine nights with 6 and 8 hours of sleep, one confirmed caffeine-free night with 8 hours, one day with unknown exposure, and one qualifying night missing sleep. Verify **7 hours across 2 nights** versus **8 hours across 1 night**, with both exclusions explained. Follow the 6-hour night to the flight observation. Save a conclusion, correct 6 hours to 8, then quit and reopen. The live caffeine mean is **8 hours**. The saved report and its Markdown export still show **7 hours**, the original two nights, and the conclusion written at the time.

## What to keep from Quiet.md

| Keep or adapt | Boundary |
| --- | --- |
| Native shell, text selection, paste, keyboard navigation, and the UI harness | Reuse throughout. Even the basic [text field](../src/input.rs) calls the editor’s rich-text renderer. Trace these dependencies before removing editor modules. |
| Quick Note window, mention picker, backlink list, and search result UI | Connect each to Ritual commands and indexed entity queries. Quiet’s document scan is unsuitable for a growing event history. |
| Block editing, undo, autosave, conflict recovery, and Markdown export | Use the small editing subset needed for experiment prose. Keep report revisions and domain writes under Ritual’s storage rules. |
| Quiet’s document library, folders, publishing layouts, guide assets, and specialized blocks | Keep these features usable in the development app. When a particular Ritual replacement is chosen, port that workflow, verify it, and retire only the route it supersedes. Check build-time embedding, shortcuts, packaging, fixtures, and harness dependencies each time. Preserve license notices and review upstream’s asset guidance before distributing. [Upstream license and asset notes](https://github.com/s1dashu/quiet-md/tree/fcd2affb7fb56b3aa9362f2c4c88bd686cc52f05#license) |

## Research and implementation references

- Ritual’s homepage and manifesto put events, context, and self-experimentation at the center. GalaxyBrain’s Days demo shows how a readable daily entry can also supply values to a summary. The plan applies this to Ritual’s existing Logs, Calendar, and Reports. General formulas and agent access can wait for a concrete workflow. The original note library continues to serve ordinary writing. [Ritual](https://ritualdb.com/) · [Manifesto](https://ritualdb.com/about) · [GalaxyBrain demo](https://app.galaxybrain.com/) · [GalaxyBrain docs](https://galaxybrain.com/docs/galaxybrain/)
- Self-tracking studies describe how unusual circumstances can be lost by the time someone interprets a pattern. Optional observations preserve that context. Dated conclusions make it possible to revisit an interpretation as more evidence arrives. [Choe et al., CHI 2014](https://www.microsoft.com/en-us/research/publication/understanding-quantified-selfers-practices-collecting-exploring-personal-data/) · [Daskalova et al., 2017](https://cs.pomona.edu/~apapoutsaki/papers/imwut2017lessons.pdf)
- In the Ritual release checkout, consult `packages/shared-contracts/src/habits.ts` and `apps/backend/models/habit_models.py` for Water, `apps/desktop/src-tauri/src/main.rs` for the voice shortcut, `packages/shared-contracts/src/entities.ts` and `apps/dashboard/components/entities/entity-note-field.tsx` for links, and `apps/backend/schemas/{experiments,artifacts}.py` for report and revision semantics.
