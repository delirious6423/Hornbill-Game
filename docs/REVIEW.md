# Hornbill review — checkpoint 0008

Your selected replacements are installed: **Gemma 12B Q6**, **Gemma E4B 8-bit**, and **Z-Image-Turbo with the exact BennyDaBall Q8_0 encoder**. Both story profiles completed four turns and a summary update; 45 automated tests pass. The superseded downloads were removed under your approval.

![The preserved turn-four scene with its new illustration](assets/hornbill-app.jpg)

Open the Hornbill tab and select **The Observatory Signal — Turn 4**. Your story, notes and Mira reference remain intact. **Settings** shows both new story choices; 12B + Smart, 512×768 and seed 42 are preserved, with nine image steps. **Story journal** shows the saved summary and world notes. There is also an unused same-title turn-0 save; all benchmark and failed-attempt records were preserved.

12B averaged **188.47 seconds per saved turn**, with all first attempts accepted. E4B averaged **85.58 seconds including its one repair**. The fresh reference illustration took **55.13 seconds**. Both story models increased system swap during these samples. These are compatibility checks, not a guarantee of long-story consistency; [validation](VALIDATION.md) includes concrete weaknesses and complete audits.

To reopen a stopped app, run `./hornbill ui` from the source directory. Its private launch link is local `data/ui-session.json`; keep that file private. A fresh clone needs the local setup in [README](../README.md); source backups omit weights, runtime installations, live saves and credentials.

The next milestone is reducing repeated decoder preparation and measuring memory pressure in a more isolated run, followed by longer story-consistency checks. Browser export-download confirmation and clean-machine packaging remain open. Read [HANDOFF](HANDOFF.md) and [WORKLOG](WORKLOG.md) before continuing.
