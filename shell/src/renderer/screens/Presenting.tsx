/**
 * Presenting.
 *
 * The deck, filling the screen, with nothing else on it. What a presenter needs while talking is a
 * slide and a way to move; what they need at hand is the note they wrote for themselves and where
 * they are in the deck. Everything else — the file list, the toolbar, the conversation — is in the
 * way and is gone.
 *
 * The keys are the ones every presenter's hands already know: space and the arrows to move, Escape
 * to come back, and the mouse only where a hand might reach for it.
 */

import { useCallback, useEffect, useRef, useState } from "react";

import { t } from "../../shared/strings.ts";
import { bridge } from "../useOwn.ts";
import { SpeechPlayer } from "../speech.ts";

export interface PresentableSlide {
  number: number;
  title: string;
  svg: string;
  notes?: string;
}

export function Presenting({
  slides,
  startAt = 0,
  onLeave,
  talk,
}: {
  slides: PresentableSlide[];
  startAt?: number;
  onLeave: () => void;
  /**
   * What to say over each slide: the deck's own notes where it has them, and the slide read as it
   * stands where it does not. Absent when the deck is being presented by the User alone.
   */
  talk?: { slide: number; words: string; fromTheDeck: boolean }[];
}) {
  const [at, setAt] = useState(Math.min(Math.max(startAt, 0), Math.max(slides.length - 1, 0)));
  // The notes are the presenter's, not the audience's. Off unless asked for.
  const [showNotes, setShowNotes] = useState(false);
  // Whether the presenting is being done aloud, and what is being said now.
  const [aloud, setAloud] = useState(false);
  const [saying, setSaying] = useState<string | undefined>();

  // The presenter is a session held open by the Core while the deck is up. Sound arrives in pieces
  // and is queued as it comes, so the voice keeps up with the slide.
  const player = useRef<SpeechPlayer | undefined>(undefined);
  const [speaking, setSpeaking] = useState(false);
  const listening = useRef(false);

  const go = useCallback(
    (by: number) => {
      setAt((current) => Math.min(Math.max(current + by, 0), Math.max(slides.length - 1, 0)));
    },
    [slides.length],
  );

  const stopTalking = useCallback(() => {
    player.current?.stop();
    setSpeaking(false);
    void bridge()?.presentHush?.();
  }, []);

  /** Take whatever the presenter has said and play it, until it stops. */
  const listen = useCallback(async () => {
    if (listening.current) return;
    listening.current = true;
    try {
      // Asked for repeatedly rather than once: the presenter is still deciding what to say while
      // the first of it is already being played.
      for (let round = 0; round < 400; round += 1) {
        const answer = (await bridge()?.presentHeard?.()) as
          | { heard?: Record<string, { base64?: string; text?: string; detail?: string }>[] }
          | undefined;
        const heard = answer?.heard ?? [];
        if (heard.length === 0) break;

        let finished = false;
        for (const one of heard) {
          if (typeof one === "string") {
            if (one === "finished") finished = true;
            continue;
          }
          if (one.sound?.base64) {
            player.current ??= new SpeechPlayer();
            player.current.add(one.sound.base64);
            setSpeaking(true);
          }
          if (one.words?.text) {
            setSaying((said) => `${said ?? ""}${one.words!.text}`);
          }
          if (one.finished !== undefined || "finished" in one) finished = true;
        }
        if (finished) break;
      }
    } finally {
      listening.current = false;
    }
  }, []);

  /** Present a slide: say its words, cutting off whatever was being said. */
  const say = useCallback(
    async (slideNumber: number) => {
      const words = talk?.find((one) => one.slide === slideNumber)?.words;
      if (!words) {
        setSaying(undefined);
        return;
      }
      // The interruption is the point of a session: moving on stops the last slide rather than
      // queueing behind it.
      player.current?.stop();
      setSaying("");
      await bridge()?.presentSay?.({ words });
      void listen();
    },
    [listen, talk],
  );

  // The keys a presenter's hands already know. On the window rather than on an element, because a
  // presenter is looking at the slide and not at whatever happens to hold the focus — losing this
  // is how the arrows stopped moving the deck.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      switch (event.key) {
        case " ":
        case "ArrowRight":
        case "ArrowDown":
        case "PageDown":
          event.preventDefault();
          go(1);
          return;
        case "ArrowLeft":
        case "ArrowUp":
        case "PageUp":
          event.preventDefault();
          go(-1);
          return;
        case "Home":
          event.preventDefault();
          setAt(0);
          return;
        case "End":
          event.preventDefault();
          setAt(Math.max(slides.length - 1, 0));
          return;
        case "Escape":
          event.preventDefault();
          stopTalking();
          onLeave();
          return;
        case "n":
        case "N":
          setShowNotes((shown) => !shown);
          return;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, onLeave, slides.length, stopTalking]);

  const slide = slides[at];

  // Said on arriving at a slide, not on every redraw, or it would start again each time the notes
  // were shown.
  useEffect(() => {
    if (!aloud || !slide) return;
    void say(slide.number);
  }, [aloud, at, say, slide]);

  // Told when the voice has stopped, so the screen does not claim it is still talking.
  useEffect(() => {
    player.current?.whenQuiet(() => setSpeaking(false));
  }, [speaking]);

  // Nothing keeps talking after the deck comes down, and the session is let go.
  useEffect(
    () => () => {
      player.current?.close();
      void bridge()?.presentEnd?.();
    },
    [],
  );

  if (!slide) {
    return (
      <div style={backdrop}>
        <p style={{ color: "#e8e5e0" }}>{t("present.nothing")}</p>
      </div>
    );
  }

  return (
    <div
      style={backdrop}
      role="region"
      aria-label={t("present.heading")}
      data-speaking={speaking ? "yes" : "no"}
    >
      {/* The slide, as large as it will go while keeping its shape. */}
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          minHeight: 0,
          padding: 18,
        }}
        // The drawn slide is the Core's own SVG, the same one the editing view shows, so what is
        // presented is what was being edited.
        dangerouslySetInnerHTML={{ __html: fitted(slide.svg) }}
      />

      {saying ? (
        <div
          aria-live="polite"
          style={{
            maxWidth: 900,
            margin: "0 auto 10px",
            color: "#f2efe9",
            fontSize: 15,
            lineHeight: 1.5,
            textAlign: "center",
          }}
        >
          {saying}
        </div>
      ) : null}

      {showNotes && slide.notes ? (
        <div
          style={{
            maxWidth: 900,
            margin: "0 auto 10px",
            padding: "10px 14px",
            background: "#1d1b18",
            border: "1px solid #33302a",
            borderRadius: 8,
            color: "#d9d5cd",
            fontSize: 13.5,
            lineHeight: 1.55,
          }}
        >
          {slide.notes}
        </div>
      ) : null}

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "8px 14px",
          borderTop: "1px solid #2a2823",
          color: "#a29d94",
          fontSize: 12,
        }}
      >
        <button type="button" onClick={() => go(-1)} style={dark} aria-label={t("present.back")}>
          ‹
        </button>
        <button type="button" onClick={() => go(1)} style={dark} aria-label={t("present.forward")}>
          ›
        </button>
        <span>
          {slide.number} / {slides.length}
        </span>
        <span style={{ color: "#6f6a62" }}>{slide.title}</span>
        {slide.notes ? (
          <button
            type="button"
            onClick={() => setShowNotes((shown) => !shown)}
            style={dark}
            aria-pressed={showNotes}
          >
            {t("present.notes")}
          </button>
        ) : null}
        {talk && talk.length > 0 ? (
          <button
            type="button"
            aria-pressed={aloud}
            onClick={() => {
              if (aloud) {
                setAloud(false);
                stopTalking();
                void bridge()?.presentEnd?.();
              } else {
                // Opened when asked for, not when the deck opens: a presenter nobody asked for
                // should not be holding a session open.
                void (async () => {
                  await bridge()?.presentBegin?.({ about: slides[0]?.title ?? "" });
                  setAloud(true);
                })();
              }
            }}
            style={dark}
          >
            {aloud ? t("present.stop_talking") : t("present.talk")}
          </button>
        ) : null}
        <button
          type="button"
          onClick={() => {
            stopTalking();
            onLeave();
          }}
          style={{ ...dark, marginLeft: "auto" }}
        >
          {t("present.leave")}
        </button>
      </div>
    </div>
  );
}

/**
 * The slide's own drawing, scaled to the space it has.
 *
 * The Core draws at a fixed width; presenting wants it as large as the screen allows without
 * stretching it, which is what a viewBox does when the width and height are given up.
 */
function fitted(svg: string): string {
  return svg.replace(
    /<svg([^>]*)>/,
    (whole, attributes: string) =>
      `<svg${attributes.replace(/\s(width|height)="[^"]*"/g, "")} style="width:100%;height:100%;max-height:82vh;object-fit:contain">`,
  );
}

const backdrop: React.CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "#141310",
  display: "flex",
  flexDirection: "column",
  zIndex: 40,
};

const dark: React.CSSProperties = {
  border: "1px solid #33302a",
  background: "#1d1b18",
  color: "#d9d5cd",
  borderRadius: 6,
  padding: "3px 9px",
  fontSize: 12,
  cursor: "pointer",
};
