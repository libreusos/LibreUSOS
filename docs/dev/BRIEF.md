# LibreUSOS: development brief (desktop/backend track)

You lead development of **LibreUSOS**, an unofficial open-source USOS client. You decide, plan, brief subagents,
review their results, and talk to the user. Follow `~/.claude/CLAUDE.md` strictly: subagent routing with `model`
and `effort` on every Agent call, the Fedora Atomic and toolbox rules, the delete rules, and code in English.
Delegate implementation, tests, builds and reviews to subagents. Stay inline only for small edits and decisions.

## Read first

- `docs/research/README.md` (summary). Use these as reference when needed:
  - `docs/research/usos-api.md`: OAuth 1.0a, scopes, modules, PolSl quirks, polling
  - `docs/research/existing-apps.md`: the official app's API method list and the feature-parity matrix
  - `docs/research/tech-stack.md`: the crate choices
- `.env` (gitignored, mode 600): `CONSUMER_KEY` and `CONSUMER_SECRET` for PolSl, `https://usosapi.polsl.pl/`.
  The CLI does not read `.env` at runtime (see PROGRESS.md).
  `APP_NAME` is only the label used when the key was registered, and the API doesn't need it.
  **Never print, log, commit or paste the secret or the tokens.** That covers tool output, test snapshots and
  error messages. They are embedded at build time by `usos-core/build.rs` (env first, `.env` fallback). Never print them or add them to build output, logs or committed files.

## Decisions already made

- App name **LibreUSOS**. Crates and binaries are prefixed `usos-`:
  - `usos-core`: API client, the in-house OAuth 1.0a HMAC-SHA1 signer, models, errors, LangDict
  - later `usos-store` (SQLite), `usos-sync`, `usos-cli`, `usos-daemon`, `usos-mcp` (rmcp), `usos-ffi`
    (UniFFI, for Android), `usos-desktop` (the GUI toolkit is still undecided)
- Decided: **only the Rust core is shared** between desktop and Android, and the **Android UI is Kotlin**
  (connected through `usos-ffi`/UniFFI). The desktop GUI is **GPUI** (crates.io `gpui` + `gpui-kit`/`gpui-component`), and it comes after the
  backend. Accessibility is not a priority. There are no desktop widgets; native KDE or GNOME widgets talking
  to the core might come much later, and only if users ask.
- This track is the **Rust backend only**: no GUI yet. Keep the core GUI-agnostic, so the CLI, daemon, MCP and
  Android can all use it.
- Base URL is entered by hand for now (later: a dropdown of all installations, with the URL as a fallback).
  UI languages are pl and en. License: **GPL-3.0-or-later** for the whole project (root `LICENSE`, `license = "GPL-3.0-or-later"` in every crate).
- Use reqwest with rustls, not openssl. Host cargo/rustc is Fedora's native toolchain, and you should use it.
- Git: local only (already `git init`ed on `main`). **No remote, never push.** Commit after each verified step,
  with small, meaningful commits.

## The user and how to work

- The user controls Ghostex **remotely** and has only Ghostex and its terminal, **no normal browser**. Ask for
  as little input as possible.
- **Login happens once, as early as possible, in the Ghostex built-in (embedded) browser pane.** Plan for the
  OAuth 1.0a flow:
  - Use a loopback callback (`http://127.0.0.1:<port>/callback`) caught by the CLI, so the user only logs in
    on the CAS page and clicks "allow". Fall back to `oob` and a PIN only if USOS rejects the loopback callback.
  - Request **all scopes you will need later, including `offline_access`**, in that single login, so the user
    never has to log in again.
  - Open the authorize URL in a Ghostex embedded browser pane via the `ghostex` CLI (`ghostex --help`, or the
    `ghostex-embedded-browser-use` skill if you can invoke it). Tell the user the one thing they must do.
  - Store the access token securely. Use Secret Service (`keyring`/`oo7`) if it is available; otherwise a
    0600 file under `~/.config/libreusos/`. It must keep working headless.
- **One thing at a time:** implement a small step, test it (unit tests, plus live calls against PolSl once
  logged in), have it **reviewed by a separate subagent** (sonnet, high), fix it, commit, then move on. Don't
  build everything in parallel.
- Keep `docs/dev/PROGRESS.md` up to date: what's done, what's verified, the next step, and decisions.
  **When your context gets large, write a handoff in PROGRESS.md and start a fresh session**
  (`ghostex agents create claude --standalone` or `ghostex create-agent claude --project-id P3l0h --model opus
  --effort high`, then `ghostex agents send` a pointer to BRIEF.md and PROGRESS.md). Tell the user it happened.
- When you need the user, say exactly what to do in one or two lines.

## Suggested first scope (adjust if you have a good reason)

1. Cargo workspace with `usos-core` and `usos-cli` (binary `usos`).
2. OAuth 1.0a signer, with unit tests against RFC 5849 / known vectors.
3. `usos login`: request_token with all scopes, embedded-browser authorize, loopback callback, access_token,
   secure storage. **Get the user's one login done here, early.**
4. `usos whoami` (`services/users/user`) and `usos raw <method> [k=v...]` (signed generic call returning
   JSON). These two prove the whole chain against PolSl.
5. Then one feature per step: terms (pick the current term by date, see the PolSl quirks), timetable
   (`tt/user`/`tt/student`, `usos plan`), grades, tests (`crstests`), and so on. After that comes the SQLite
   cache (`usos-store`), then `usos-mcp`.

There is no main coordinating session any more (the user closed it on 2026-11-06 and wants no relay).
Report milestones and blockers to the user in your own chat, in 3 lines, and keep PROGRESS.md current.
