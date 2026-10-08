# Surface brief: ground station app shell (`src-ui/src/App.svelte` + components)

Scope: the single operator screen of the Tauri app (all panels, the session dialog, status bar).
Mode: Operate. Secondary audience: judges viewing a poster/projection during preliminary review.
Constraint: default Tauri window is 1200×800; must also hold at 1440×900 and 1920×1080. Offline except map tiles. CSP allows only self-hosted fonts.

## Direction contract

THESIS: The screen is read like an aeronautical sectional chart: a calm chart-paper field where the only loud ink is the rocket itself. It refuses the category default of a dark glass dashboard with uniform rounded cards, English eyebrow labels and teal glow.

OWN-WORLD: Light chart-paper ground (cool grey-green), navy ink, a committed chart-blue rail that owns the left edge, hairline rules instead of cards, aeronautical magenta reserved for the rocket (altitude trace, marker, track, peak label). Hypsometric tint ramp (green → yellow → tan → brown) as the altitude scale. Night variant: deep navy ground, same roles. Barlow (DIN-family road/chart lettering) for all numerals; Microsoft JhengHei for Chinese. No monospace costume, no pills, no glass.

STORY: The operator sees at a glance: is the link live, is it recording, how high is the rocket, is deploy SAFE. Judges see a confident, legible instrument with a distinctive chart identity.

FIRST VIEWPORT: Rail 248px (product mark, current session title, link-state block, start/stop, COM/protocol, axis settings). Center: altitude readout at ~120px numerals beside a vertical hypsometric ladder with a magenta pin; four secondary readings in a ruled column on the right; below, the altitude profile drawn on tint bands against airborne uptime; bottom row attitude + full 13-field table. Right column: map with launch-relative distance/bearing strip, then flight control (SAFE badge, remaining countdown, timer override, two-step arm/fire). Diagnostics strip along the bottom.

SIGNATURE: 高度色階 — the hypsometric tint ladder; the same bands sit behind the profile chart, and the pin travels with altitude.

FORM: Sectional chart (aeronautical), position 1 on the ordered list; chosen by the user from three rendered previews (no concept-seed key: launcher not run in this session).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
