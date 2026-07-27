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
  const playing = useRef<HTMLAudioElement | null>(null);
  const [speaking, setSpeaking] = useState(false);

  const stopTalking = useCallback(() => {
    playing.current?.pause();
    playing.current = null;
    setSaying(undefined);
    setSpeaking(false);
  }, []);

  /** Say the words for a slide, stopping whatever was being said. */
  const say = useCallback(
    async (slideNumber: number) => {
      const words = talk?.find((one) => one.slide === slideNumber)?.words;
      if (!words) {
        setSaying(undefined);
        return;
      }
      stopTalking();
      setSaying(words);
      const answer = (await bridge()?.speak?.({ words })) as
        | { wav?: string; problem?: string }
        | undefined;
      if (!answer?.wav) {
        // Said on screen even when it cannot be said aloud, so a presenter is not left with a
        // silent slide and no idea why.
        return;
      }
      const sound = new Audio(`data:audio/wav;base64,${answer.wav}`);
      playing.current = sound;
      // Recorded on the surface so it can be seen from outside that the words are being said and
      // not merely shown — a silent presentation that claims to be talking is the failure worth
      // catching.
      sound.addEventListener("playing", () => setSpeaking(true));
      sound.addEventListener("ended", () => setSpeaking(false));
      void sound.play().catch(() => setSpeaking(false));
    },
    [stopTalking, talk],
  );

  const go = useCallback(
    (by: number) => {
      setAt((current) => Math.min(Math.max(current + by, 0), Math.max(slides.length - 1, 0)));
    },
    [slides.length],
  );

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
          onLeave();
          return;
        case "n":
        case "N":
          // The presenter's own notes, on the presenter's own screen.
          setShowNotes((shown) => !shown);
          return;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, onLeave, slides.length]);

  const slide = slides[at];

  // Said on arriving at a slide, not on every redraw, or it would start again each time the notes
  // were shown.
  useEffect(() => {
    if (!aloud || !slide) return;
    void say(slide.number);
  }, [aloud, at, say, slide]);

  // Nothing keeps talking after the presenting stops.
  useEffect(() => stopTalking, [stopTalking]);

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
              } else {
                setAloud(true);
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
