// ---------------------------------------------------------------------------
// Fun Facts copy
//
// Static PT-BR He4rt facts the Footer Bar's ticker scrolls through. Kept as a
// plain constant (no feed coupling) so FunFactsTicker stays a pure props sink.
// ---------------------------------------------------------------------------

export const FUN_FACTS: string[] = [
  "A He4rt Developers foi fundada em 2018 por Daniel Reis 💜",
  "Tudo começou num canal de LiveCoding na Twitch",
  "Da comunidade para a comunidade 💜",
  "Mais de 27 mil devs reunidos no Discord",
  "Projetos abertos: 4noobs, He4rt Delas e He4rt Live",
  "danielhe4rt é Microsoft MVP e Twitch Partner",
  "Stack da casa: PHP, Laravel, Rust e PostgreSQL",
  "He4rt Delas fortalece mulheres na tecnologia",
  "Aprender em público é o nosso lema 🚀",
  "Entra no discord.gg/he4rt e bora codar!",
];

// Brand / project / tech terms the ticker accents in the facts above. Numbers
// and URLs (discord.gg/…) are highlighted automatically by the ticker; this list
// covers the named entities. Passed to <FunFactsTicker highlightTerms={…} /> so
// the component itself stays content-agnostic.
export const FUN_FACT_HIGHLIGHTS: string[] = [
  "He4rt Developers",
  "He4rt Delas",
  "He4rt Live",
  "Daniel Reis",
  "danielhe4rt",
  "4noobs",
  "Microsoft MVP",
  "Twitch Partner",
  "LiveCoding",
  "PostgreSQL",
  "Laravel",
  "Rust",
  "PHP",
  "Discord",
  "Twitch",
];
