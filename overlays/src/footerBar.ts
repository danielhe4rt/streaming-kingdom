// ---------------------------------------------------------------------------
// Footer Bar arbitration (issue #6)
//
// The Frame Overlay's Footer Bar arbitrates between Fun Facts (idle rotation)
// and Alerts (a StreamEvent taking over the bar). Alerts QUEUE and play one at
// a time so a burst never drops one. The state machine (from the layout
// prototype) is:
//
//   state: { mode: 'funfacts' | 'alert', queue: StreamEvent[], current }
//   on StreamEvent e        -> queue.push(e); if mode==='funfacts' playNext()
//   playNext()              -> current = queue.shift(); mode = current ? 'alert' : 'funfacts'
//   on alert animation end  -> playNext()   // empty queue => back to funfacts
//
// Kept as a pure reducer + a thin hook so the arbitration is testable in
// isolation and the component just dispatches.
// ---------------------------------------------------------------------------

import { useCallback, useReducer } from "react";
import type { StreamEventDto } from "./feed";

export type FooterMode = "funfacts" | "alert";

export interface FooterBarState {
  mode: FooterMode;
  queue: StreamEventDto[];
  current: StreamEventDto | null;
}

export type FooterBarAction =
  | { type: "streamEvent"; event: StreamEventDto }
  | { type: "alertEnded" };

export const initialFooterBarState: FooterBarState = {
  mode: "funfacts",
  queue: [],
  current: null,
};

// playNext: pull the head of the queue into `current`. An empty queue returns
// the bar to Fun Facts; otherwise it plays the next Alert.
function playNext(state: FooterBarState): FooterBarState {
  if (state.queue.length === 0) {
    return { mode: "funfacts", queue: [], current: null };
  }
  const [next, ...rest] = state.queue;
  return { mode: "alert", queue: rest, current: next };
}

export function footerBarReducer(
  state: FooterBarState,
  action: FooterBarAction,
): FooterBarState {
  switch (action.type) {
    case "streamEvent": {
      // Always enqueue so a burst never drops an Alert. If the bar is idle,
      // start playing immediately; if an Alert is already on screen, the new
      // one just waits its turn (drained on alertEnded).
      const queued: FooterBarState = {
        ...state,
        queue: [...state.queue, action.event],
      };
      return queued.mode === "funfacts" ? playNext(queued) : queued;
    }
    case "alertEnded": {
      // The current Alert finished its animation: advance the queue.
      return playNext(state);
    }
    default:
      return state;
  }
}

export interface FooterBar {
  state: FooterBarState;
  pushEvent: (event: StreamEventDto) => void;
  endAlert: () => void;
}

/** React hook wrapping the Footer Bar arbitration reducer. */
export function useFooterBar(): FooterBar {
  const [state, dispatch] = useReducer(footerBarReducer, initialFooterBarState);

  const pushEvent = useCallback((event: StreamEventDto) => {
    dispatch({ type: "streamEvent", event });
  }, []);

  const endAlert = useCallback(() => {
    dispatch({ type: "alertEnded" });
  }, []);

  return { state, pushEvent, endAlert };
}
