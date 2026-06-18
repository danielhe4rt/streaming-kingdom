# TUI layout prototype — verdict

**Pergunta:** como reorganizar a UI do control-panel pra ser escalável e robusta agora
que precisa hospedar o controle de **Overlays** ao lado de services / stats / chat / event log
— sem o grid fixo de 4 panes nem o mapeamento de cursor por número mágico?

**Arquivo:** `tui-layout-prototype.html` (throwaway). 3 variantes via `?variant=`.

- **A — Sidebar Nav:** navega 1 seção por vez (Dashboard / Services / Overlays / Chat / Events).
  Escala infinito (nova seção = item na sidebar). Menos densidade num glance.
- **B — Dense Dashboard:** tudo visível, reagrupado (KPIs no topo, Services | Chat | Overlays+Highlights,
  Event Log full-width). Bom pra monitor único. Pode apertar quando crescer.
- **C — Master / Detail:** lista de módulos (Inputs/Outputs) + inspector à direita, chat **pinned**.
  Dá espaço pra config por-overlay no futuro. Drill-in em 1 coisa por vez.
- **D — Topbar > Sidebar > Content:** nav em 2 níveis. Topbar = seção primária
  (Dashboard / Services / Overlays / Activity); sidebar = sub-nav contextual
  (ex: Overlays → All / Chat overlay / Frame overlay / Feed); content = painel.
  Escala em duas dimensões; mais cliques pra chegar, mas hierarquia clara.

Insight transversal nas 3: services modelados como **Inputs vs Outputs**, e Overlays é um Output.
Tudo renderiza de um `state` único via "component builders" → adicionar serviço/overlay = 1 entrada.

---

## VEREDITO

- **Variante escolhida: D — Topbar > Sidebar > Content** (nav em 2 níveis).
- Por quê: mais escalável (seção × sub-seção), hierarquia clara, e dá o lugar
  mais limpo pra Overlays (top-level) com sub-nav All / Chat / Frame / Feed —
  inclusive espaço pra config por-overlay no futuro.
- Implica redesenhar a TUI ratatui de "4 panes + Tab" para um **nav shell**:
  tab bar horizontal (topbar) + lista de sub-nav (sidebar) + painel de conteúdo.
