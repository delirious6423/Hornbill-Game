# Hornbill review — checkpoint 0010

The local MVP is ready to inspect with your selected **Gemma 12B Q6**, **Gemma E4B 8-bit** and **Z-Image-Turbo/BennyDaBall Q8_0** models. All **50 automated tests** pass, including a clean source/dependency rehearsal. Superseded model downloads were removed under your approval.

![The preserved review scene](assets/hornbill-app.jpg)

Open the Hornbill tab and select **The Observatory Signal — Turn 4**. Your story, notes, reference and illustration are intact. **Settings** chooses the model and illustration schedule. **Story journal** shows the summary/world notes and exports the story with its generation record. Accepted turns save automatically. Benchmarks and failed attempts remain in the saved-story list.

The new decoder reduced average four-turn results to **112 seconds per saved turn for 12B**, and **46 seconds for E4B including one repair**. The reference illustration took **55 seconds**. 12B still increased swap and briefly reached warning memory pressure. E4B is faster but showed weaker story continuity. The eight-turn 12B check passed state/summary validation while still repeating dialogue and introducing unsupported narrative details. Read [validation](VALIDATION.md) for the measurements and concrete limits.

To reopen a stopped app, run `./hornbill ui` from this source directory. Keep the launching Terminal open. New installations use the steps in [README](../README.md). The app runs entirely locally after setup. Source backups contain code, documentation and test evidence; weights, runtime installations, live saves and private session keys stay local.

Save/export consistency, browser download, bounded long-run retrieval and clean dependencies are checked. Future work can focus on story quality, optional native packaging or stronger reference identity. The [handoff](HANDOFF.md) and [worklog](WORKLOG.md) contain the final checkpoint and exact resume paths.
