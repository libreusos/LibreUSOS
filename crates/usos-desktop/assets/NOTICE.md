# Bundled assets

These files are embedded in the `usos-desktop` binary.

| Path | What | Licence |
| --- | --- | --- |
| `fonts/Geist-{400,500,600}.ttf`, `fonts/GeistMono-{400,500}.ttf` | Static desktop instances (Basic Latin, Latin-1, Latin Extended-A, punctuation, arrows, maths signs) built from the Geist and Geist Mono variable fonts by the Geist Project Authors | SIL Open Font License 1.1, see `fonts/OFL.txt` |
| `icons/*.svg` | Material Symbols Outlined, weight 400, grade 0, sizes 20 and 24 (plain and `fill1`), from https://github.com/google/material-design-icons | Apache License 2.0, see `icons/LICENSE-APACHE-2.0.txt` |
| `logo/*.svg` | LibreUSOS logo mark | GPL-3.0-or-later, with the project |

The Lucide icons that gpui-component draws inside its own widgets (checkmarks, chevrons, close
buttons) come from the `gpui-kit-assets` crate, licensed ISC (Lucide) and Apache-2.0.
