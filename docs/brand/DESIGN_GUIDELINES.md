# Dimvexa Design Guidelines

Version 1.0 | 10 November 2026

## Brand concept

Dimvexa is an exact unit-conversion library. The mark consists of two interlocking, opposed geometric arrows: **bidirectional conversion** with a forward-moving silhouette to suggest **speed**. The precise angular construction communicates reliable, deterministic software. Avoid adding motion streaks or small decorative elements.

## Colour palette

| Role | HEX | RGB | Usage |
| --- | --- | --- | --- |
| Charcoal | `#181C24` | 24, 28, 36 | Primary dark surfaces and app-icon tile |
| Sky blue | `#60A5FA` | 96, 165, 250 | Upper/right arrow, links, highlights |
| Electric blue | `#2563EB` | 37, 99, 235 | Lower/left arrow, active accents |
| Off-white | `#F1F5F9` | 241, 245, 249 | Main text on dark surfaces |
| Muted slate | `#94A3B8` | 148, 163, 184 | Secondary text on dark surfaces |
| Light canvas | `#F8FAFC` | 248, 250, 252 | Light-mode background |

Keep the mark in the two specified blues. Do not introduce gradients, shadows, bevels, glows, or amber accents. Blue arrows on a dark or very light solid surface are the primary treatment.

## Typography

Use **Inter** when available, with **Arial / Helvetica / system sans-serif** as fallbacks. Wordmark: bold (700-750), slightly tightened tracking; headings: 600-700; body: 400-500. Keep technical labels and supporting captions understated. The SVG lockups contain editable text and depend on local font availability; convert text to outlines before sending to a vendor requiring pixel-identical reproduction.

## Logo usage and clear space

- Preferred lockup: symbol left of `Dimvexa`, off-white lettering on charcoal.
- Alternate lockup: dark lettering on a light canvas.
- Use `dimvexa-mark.svg` for an icon without a background; use `dimvexa-icon.svg` for the square app icon.
- Maintain clear space around the logo of at least **10% of the symbol width** on every side.
- Do not stretch, rotate, recolour the arrows independently, or place the logo on a noisy image.
- Keep the rounded tile fully within crop boundaries. Do not add another enclosing circle.

## Small-size rules

- At **16 px**, use `icon-16.png` or `favicon.svg`. Avoid accompanying text inside the icon.
- At **32 px**, check both arrows remain visibly separated at actual display size.
- At **80 px** and above, use the standard icon with no added details.
- Render SVG at target pixel dimensions; avoid scaling a small PNG upward.

## Asset mapping

| Context | Recommended asset |
| --- | --- |
| Excel add-in | `icon-32.png`, `icon-80.png` |
| Browser favicon | `favicon.svg`, `icon-16.png`, `icon-32.png`, `icon-48.png` |
| npm / PyPI avatar | `icon-256.png` or `icon-512.png` |
| README heading | `dimvexa-lockup-dark.svg` on dark background or `dimvexa-lockup-light.svg` on light background |
| GitHub social preview | `github-social-preview.png` (1280 × 640) |
| Scalable master | `dimvexa-mark.svg`, `dimvexa-icon.svg` |

## Accessibility and contrast

Use off-white for primary text on charcoal and muted slate for secondary text. The two arrow colours are brand accents, not a substitute for accessible text colours. Check colour contrast for any new text-on-blue combinations and confirm recognisability at 100% zoom in light and dark themes.

## Implementation notes

The master icons are original, hand-constructed vector paths with flat fills. The PNGs are exported directly from SVG at their final pixel dimensions. The social preview is a scalable SVG with a 1280 × 640 PNG export. Keep the SVG master under version control and regenerate PNGs after any geometry change.
