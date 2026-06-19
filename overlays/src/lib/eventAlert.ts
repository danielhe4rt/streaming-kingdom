// ---------------------------------------------------------------------------
// Event-alert view-model mapper
//
// Turns a feed StreamEventDto into the props the Footer Bar's EventAlert card
// renders. The ui/ layer stays headless: all the PT-BR copy, icons and accent
// hexes (pulled from our @theme tokens) live here, keyed off the discriminant.
// viewerCountUpdate is intentionally silent — it never surfaces as an alert.
// ---------------------------------------------------------------------------

import type { StreamEventDto } from "../feed";
import type { EventAlertProps } from "../ui/footer/EventAlert";

export function toEventAlert(e: StreamEventDto): EventAlertProps | null {
  switch (e.type) {
    case "follow":
      return {
        icon: "⭐",
        accent: "#c9a4ff", // brand-light
        title: "NOVO FOLLOW",
        name: "@" + e.username,
        detail: "",
      };
    case "sub":
      return {
        icon: "💜",
        accent: "#8b2fe8", // brand
        title: "NOVO SUB",
        name: "@" + e.username,
        detail: e.months > 1 ? e.months + " meses" : "",
      };
    case "donation":
      return {
        icon: "💸",
        accent: "#1ed760", // spotify
        title: "DOAÇÃO",
        name: "@" + e.username,
        detail: "R$ " + (e.amountCents / 100).toFixed(2),
      };
    case "giftSub":
      return {
        icon: "🎁",
        accent: "#8b2fe8", // brand
        title: "GIFT SUB",
        name: "@" + e.username,
        detail: e.total + " subs",
      };
    case "cheer":
      return {
        icon: "💎",
        accent: "#ffcb05", // yellow
        title: "BITS",
        name: "@" + e.username,
        detail: e.bits + " bits",
      };
    case "raid":
      return {
        icon: "⚡",
        accent: "#ffcb05", // yellow
        title: "RAID",
        name: "@" + e.fromChannel,
        detail: "+" + e.viewers + " viewers",
      };
    case "viewerCountUpdate":
      // Silent metric — never alerts.
      return null;
  }
}
