# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to
the actual label strings used in this repo's issue tracker, and adds two extra label tiers
(`area:*` and `infra:*`) derived from the codebase's DDD structure so issues route to the
right module.

## 1. Canonical triage roles (state machine)

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the
corresponding label string from this table.

## 2. Area labels (DDD layer — one per top-level `src/` module)

Apply exactly one to indicate which architectural layer owns the issue.

| Label                | Layer (`src/…`)    | Scope                                                         |
| -------------------- | ------------------ | ------------------------------------------------------------ |
| `area:domain`        | `src/domain/`      | Core types: events, chat, commands, stats, app_event         |
| `area:application`   | `src/application/` | Config, state, orchestration wiring                          |
| `area:infrastructure`| `src/infrastructure/` | External integrations (see `infra:*` below)               |
| `area:presentation`  | `src/presentation/`| TUI: input, state, theme, ui                                 |

## 3. Infra labels (external integration — one per `src/infrastructure/` submodule)

Add alongside `area:infrastructure` when the issue is specific to one integration.

| Label             | Module (`src/infrastructure/…`) | Scope                              |
| ----------------- | ------------------------------- | ---------------------------------- |
| `infra:twitch`    | `twitch/`                       | IRC, EventSub, auth                |
| `infra:elevenlabs`| `elevenlabs/`                   | TTS                                |
| `infra:hyprland`  | `hyprland/`                     | Event listener, privacy monitor    |
| `infra:livepix`   | `livepix/`                      | Donation webhooks                  |
| `infra:obs`       | `obs/`                          | OBS control                        |
| `infra:waybar`    | `waybar/`                       | Waybar config, events, style       |

### Example

A bug in Twitch EventSub →
`needs-triage` + `area:infrastructure` + `infra:twitch`.

---

Edit any right-hand column to match the vocabulary you actually configure in GitHub. Create
the labels once with `gh label create "<name>" --description "..." --color "<hex>"`.
