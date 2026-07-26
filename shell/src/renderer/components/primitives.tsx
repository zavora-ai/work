/**
 * Interface primitives.
 *
 * Small and deliberately few. Every state indicator pairs a colour role with a
 * distinct glyph shape and a text label, so nothing is carried by colour alone
 * (Requirement 21.4).
 */

import type { ChangeEvent, CSSProperties, KeyboardEvent, ReactNode } from "react";

import { t, type StringKey } from "../../shared/strings.ts";

/* ------------------------------------------------------------------ icons */

const PATHS = {
  dashboard: "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
  plus: "M12 5v14M5 12h14",
  settings:
    "M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2v.2a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-2.9-1.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 2.5 15a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.2-2.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 2.9-1.2V4a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 2.9 1.2l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1A1.7 1.7 0 0 0 21.5 11h.2a2 2 0 1 1 0 4z",
  document: "M14 3v5h5M19 8v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7z",
  deck: "M2 4h20v13H2zM9 21h6M12 17v4",
  sheet: "M3 4h18v16H3zM3 10h18M9 10v10",
  folder: "M3 7h6l2 2h10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  clock: "M12 7v5l3 2",
  warning: "M12 3 2 20h20L12 3z",
  info: "M12 8h.01M11 12h1v5h1",
  question: "M9.1 9a3 3 0 1 1 4.6 2.5c-.9.6-1.7 1.1-1.7 2.5M12 18h.01",
  check: "M20 6 9 17l-5-5",
  pause: "M9 5v14M15 5v14",
  chevronLeft: "M15 6l-6 6 6 6",
  chevronRight: "M9 6l6 6-6 6",
  arrowRight: "M0 7h20M15 2l5 5-5 5",
  lock: "M8 11V8a4 4 0 0 1 8 0v3",
  bolt: "M13 2 3 14h7l-1 8 10-12h-7z",
  chat: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4.3-4.3",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({
  name,
  size = 15,
  stroke = "currentColor",
  width = 2,
}: {
  name: IconName;
  size?: number;
  stroke?: string;
  width?: number;
}) {
  const circle = name === "clock" || name === "info" || name === "question";
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={stroke}
      strokeWidth={width}
      strokeLinecap="round"
      aria-hidden="true"
      style={{ flex: "0 0 auto" }}
    >
      {circle ? <circle cx="12" cy="12" r="9" /> : null}
      {name === "lock" ? <rect x="4" y="11" width="16" height="10" rx="2" /> : null}
      <path d={PATHS[name]} />
    </svg>
  );
}

/* ------------------------------------------------------------------ buttons */

export function Button({
  children,
  primary = false,
  small = false,
  onClick,
  disabled,
  style,
  title,
}: {
  children: ReactNode;
  primary?: boolean;
  small?: boolean;
  onClick?: () => void;
  /** A control that cannot act says so, rather than accepting a press and doing nothing. */
  disabled?: boolean;
  style?: CSSProperties;
  title?: string;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      title={title}
      style={{
        border: "1px solid",
        borderColor: primary ? "#26241f" : "var(--border-strong)",
        background: primary ? "#26241f" : "#fff",
        color: primary ? "#fff" : "#33302a",
        borderRadius: "var(--radius-sm)",
        padding: small ? "3px 9px" : "5px 12px",
        fontSize: small ? 12 : 12.5,
        fontWeight: 560,
        whiteSpace: "nowrap",
        // A control that cannot act must look as though it cannot, or the User presses it and
        // concludes the product is broken.
        cursor: disabled ? "default" : "pointer",
        opacity: disabled ? 0.5 : 1,
        ...style,
      }}
    >
      {children}
    </button>
  );
}

/* ------------------------------------------------------------------ surfaces */

export function Card({
  children,
  style,
  edge,
}: {
  children: ReactNode;
  style?: CSSProperties;
  /** A left edge colour, used by tray items alongside their glyph and label. */
  edge?: string;
}) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderLeftWidth: edge ? 3 : 1,
        borderLeftColor: edge ?? "var(--border)",
        borderRadius: "var(--radius)",
        padding: "12px 15px",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

export function Pill({
  children,
  tone = "neutral",
  icon,
}: {
  children: ReactNode;
  tone?: "live" | "warn" | "neutral";
  icon?: IconName;
}) {
  const tones = {
    live: { bg: "var(--live-bg)", fg: "var(--live-fg)" },
    warn: { bg: "var(--warn-bg)", fg: "var(--warn-fg)" },
    neutral: { bg: "#eceae5", fg: "#6b665e" },
  } as const;
  const { bg, fg } = tones[tone];
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        background: bg,
        color: fg,
        fontSize: 11,
        fontWeight: 650,
        padding: "2px 8px",
        borderRadius: 20,
      }}
    >
      {icon ? <Icon name={icon} size={10} width={2.6} /> : null}
      {children}
    </span>
  );
}

export function Chip({
  children,
  on = false,
  onClick,
}: {
  children: ReactNode;
  on?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={on}
      style={{
        fontSize: 12,
        fontWeight: 560,
        padding: "4px 11px",
        borderRadius: 20,
        border: "1px solid",
        borderColor: on ? "#26241f" : "var(--border-strong)",
        background: on ? "#26241f" : "#fff",
        color: on ? "#fff" : "var(--ink-soft)",
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

/** A read-only field, standing in for a real input until the plumbing lands. */
export function Field({
  placeholder,
  value,
  mono = false,
  style,
  onChange,
  onKeyDown,
  disabled,
  label,
}: {
  placeholder?: string;
  value?: string;
  mono?: boolean;
  style?: CSSProperties;
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
  disabled?: boolean;
  /** An accessible name, for a field with no visible label of its own. */
  label?: string;
}) {
  const frame: CSSProperties = {
    border: "1px solid var(--border-strong)",
    borderRadius: 8,
    padding: "9px 11px",
    fontSize: 13,
    background: "#fdfdfc",
    fontFamily: mono ? "ui-monospace, SFMono-Regular, Menlo, monospace" : "inherit",
    ...style,
  };

  // A field the User can type into is a real input, so the keyboard, screen readers and
  // the browser's own text handling all work. One that only shows a value stays a plain
  // box, which is what the formula bar and the read-only screens want.
  if (onChange) {
    return (
      <input
        type="text"
        value={value ?? ""}
        placeholder={placeholder}
        aria-label={label ?? placeholder}
        disabled={disabled}
        onChange={onChange}
        onKeyDown={onKeyDown}
        style={{ ...frame, color: "#3d3933", width: "100%", boxSizing: "border-box" }}
      />
    );
  }

  return (
    <div style={{ ...frame, color: value ? "#3d3933" : "var(--faint)" }}>
      {value ?? placeholder}
    </div>
  );
}

export function Toggle({ on, label }: { on: boolean; label: string }) {
  return (
    <span
      role="switch"
      aria-checked={on}
      aria-label={label}
      title={label}
      tabIndex={0}
      style={{
        width: 36,
        height: 21,
        borderRadius: 20,
        background: on ? "var(--live-fg)" : "var(--border-strong)",
        position: "relative",
        display: "inline-block",
        flex: "0 0 auto",
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 2.5,
          left: on ? 17.5 : 2.5,
          width: 16,
          height: 16,
          borderRadius: "50%",
          background: "#fff",
        }}
      />
    </span>
  );
}

export function Segmented({
  options,
  active,
  onSelect,
  small = false,
}: {
  options: string[];
  active: string;
  onSelect?: (value: string) => void;
  small?: boolean;
}) {
  return (
    <div
      role="tablist"
      style={{
        display: "inline-flex",
        gap: 2,
        background: "#f1efea",
        borderRadius: 8,
        padding: 2,
      }}
    >
      {options.map((option) => {
        const on = option === active;
        return (
          <button
            key={option}
            type="button"
            role="tab"
            aria-selected={on}
            onClick={() => onSelect?.(option)}
            style={{
              border: 0,
              fontSize: small ? 11.5 : 12,
              fontWeight: 600,
              color: on ? "#26241f" : "#6d6862",
              padding: small ? "4px 10px" : "5px 13px",
              borderRadius: 6,
              background: on ? "#fff" : "transparent",
              boxShadow: on ? "0 1px 2px rgba(0,0,0,.06)" : "none",
              cursor: "pointer",
            }}
          >
            {option}
          </button>
        );
      })}
    </div>
  );
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        fontSize: 10.5,
        fontWeight: 700,
        letterSpacing: ".07em",
        textTransform: "uppercase",
        color: "var(--faint)",
        padding: "14px 9px 5px",
      }}
    >
      {children}
    </div>
  );
}

/** A string from the catalogue. Components use this rather than a literal. */
export function T({ k }: { k: StringKey }) {
  return <>{t(k)}</>;
}
