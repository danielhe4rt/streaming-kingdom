# Context Map — streams-toolkit

This repo is organised as a DDD-layered Rust crate. Each architectural layer is its own
bounded context with a dedicated `CONTEXT.md`. Skills read this map first, then the relevant
context.

| Context        | Glossary file                      | Responsibility                                                         |
| -------------- | ---------------------------------- | ---------------------------------------------------------------------- |
| Domain         | `src/domain/CONTEXT.md`            | Core event/chat/command/stats types — the language of the system       |
| Application    | `src/application/CONTEXT.md`       | Config, shared state, orchestration of domain + infrastructure         |
| Infrastructure | `src/infrastructure/CONTEXT.md`    | Adapters to external systems (Twitch, ElevenLabs, Hyprland, …)         |
| Presentation   | `src/presentation/CONTEXT.md`      | Terminal UI — input, view state, theme, rendering                      |

System-wide architectural decisions live in `docs/adr/`. Context-specific decisions live in
`src/<layer>/docs/adr/` when they exist.

> These `CONTEXT.md` files are currently stubs. `/grill-with-docs` fills in glossary terms and
> ADRs lazily, as decisions get resolved — don't pre-populate them speculatively.
