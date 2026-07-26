/**
 * The User's own folder.
 *
 * This screen used to show fourteen invented files under the claim "Folders here are real
 * folders on your Mac". It now shows what is actually on disk, and a file opens the
 * workspace for its kind. Folders are real folders and kinds are filters, never containers,
 * so a filter narrows the list and never rearranges it into groups that do not exist
 * (Correctness Property 31).
 */

import { useMemo, useState } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Chip, Field, Icon } from "../components/primitives.tsx";
import { Empty } from "../components/states.tsx";
import { useFolder, type Entry } from "../useOwn.ts";
import type { Route } from "../routes.ts";

const COLUMNS = "2.4fr 1.2fr 1fr 90px";

const KINDS = [
  { key: "everything", label: t("repo.kind.everything") },
  { key: "document", label: t("repo.kind.documents") },
  { key: "deck", label: t("repo.kind.decks") },
  { key: "spreadsheet", label: t("repo.kind.spreadsheets") },
  { key: "pdf", label: t("repo.kind.pdfs") },
];

const ROUTE_KIND: Partial<Record<Route, string>> = {
  documents: "document",
  decks: "deck",
  spreadsheets: "spreadsheet",
};

/** When it changed, in the words someone would use out loud. */
function whenChanged(seconds?: number): string {
  if (!seconds) return "—";
  const then = new Date(seconds * 1000);
  const minutes = Math.floor((Date.now() - then.getTime()) / 60000);
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return then.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const days = Math.floor(hours / 24);
  if (days === 1) return "Yesterday";
  if (days < 7) return then.toLocaleDateString([], { weekday: "long" });
  return then.toLocaleDateString([], { day: "numeric", month: "short" });
}

function howBig(bytes?: number): string {
  if (bytes === undefined) return "—";
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function Repository({
  onNavigate,
  route = "repository",
  onOpenFile,
  onOpenPath,
}: {
  onNavigate: (route: Route) => void;
  route?: Route;
  onOpenFile?: () => void;
  /** Open one of the User's own files. */
  onOpenPath?: (path: string, kind?: string) => void;
}) {
  const [kind, setKind] = useState<string>(ROUTE_KIND[route] ?? "everything");
  const [within, setWithin] = useState<string | undefined>(undefined);
  const [search, setSearch] = useState("");
  const { state, newFolder } = useFolder(within);

  // Folders always show, because they are how the User navigates. That means a kind filter
  // can produce a list of folders and no files, which reads as a result when it is not one —
  // so matching files are counted separately and the emptiness is said plainly.
  const rows = useMemo(() => {
    const term = search.trim().toLowerCase();
    return state.entries.filter((entry) => {
      if (term && !entry.name.toLowerCase().includes(term)) return false;
      return kind === "everything" || entry.isFolder || entry.kind === kind;
    });
  }, [state.entries, kind, search]);

  const matchingFiles = rows.filter((entry) => !entry.isFolder).length;
  const files = state.entries.filter((entry) => !entry.isFolder).length;
  const folders = state.entries.length - files;

  const open = (entry: Entry) => {
    if (entry.isFolder) {
      setWithin(entry.path);
      return;
    }
    if (!entry.kind) return;
    onOpenPath?.(entry.path, entry.kind);
  };

  return (
    <main className="main">
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
        <h1 className="h1" style={{ margin: 0 }}>
          {kind === "everything"
            ? t("repo.all_files")
            : (KINDS.find((option) => option.key === kind)?.label ?? t("repo.all_files"))}
        </h1>
        <div className="ml" style={{ display: "flex", gap: 7 }}>
          {within ? (
            <Button small onClick={() => setWithin(undefined)}>
              {t("repo.back_to_top")}
            </Button>
          ) : null}
          {onOpenFile && (
            <Button small onClick={onOpenFile}>
              {t("new.open_a_file")}
            </Button>
          )}
          <Button
            small
            onClick={() => {
              const name = window.prompt(t("repo.new_folder_name"));
              if (name) void newFolder(name);
            }}
          >
            {t("repo.new_folder")}
          </Button>
        </div>
      </div>

      {/* The location and the counts are what is really there, not a fixed sentence. */}
      <p style={{ fontSize: 12, color: "var(--muted)", margin: "0 0 12px" }}>
        <b style={{ color: "#26241f" }}>{state.location ?? "—"}</b>
        {state.problem
          ? ` — ${state.problem}`
          : ` — ${files} ${files === 1 ? "file" : "files"}, ${folders} ${
              folders === 1 ? "folder" : "folders"
            } on your Mac`}
      </p>

      <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 14 }}>
        {KINDS.map((option) => (
          <Chip key={option.key} on={kind === option.key} onClick={() => setKind(option.key)}>
            {option.label}
          </Chip>
        ))}
        <Field
          placeholder={t("repo.search")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          style={{ marginLeft: "auto", width: 150, padding: "5px 10px", fontSize: 12 }}
        />
      </div>

      {!state.loading && (rows.length === 0 || matchingFiles === 0) ? (
        <Empty
          icon="folder"
          title={
            state.entries.length === 0 ? t("repo.nothing_yet") : t("repo.nothing_of_that_kind")
          }
          what={t("repo.where_files_live")}
        />
      ) : null}

      <div
        style={{
          background: "var(--card)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius)",
          overflow: "hidden",
          display: rows.length === 0 ? "none" : "block",
          marginTop: matchingFiles === 0 ? 12 : 0,
        }}
      >
        <div
          style={{
            display: "grid",
            gridTemplateColumns: COLUMNS,
            padding: "9px 15px",
            background: "#fbfaf8",
            borderBottom: "1px solid var(--border)",
            fontSize: 10.5,
            fontWeight: 700,
            letterSpacing: ".06em",
            textTransform: "uppercase",
            color: "var(--faint)",
          }}
        >
          <div>{t("repo.col.name")}</div>
          <div>{t("repo.col.changed")}</div>
          <div>{t("repo.col.size")}</div>
          <div />
        </div>
        {rows.map((entry, index) => (
          <Row
            key={entry.path}
            entry={entry}
            last={index === rows.length - 1}
            onOpen={() => open(entry)}
          />
        ))}
      </div>

      <p className="hint" style={{ marginTop: 14 }}>
        {t("repo.real_folders")}
      </p>
    </main>
  );
}

function Row({
  entry,
  last,
  onOpen,
}: {
  entry: Entry;
  last: boolean;
  onOpen: () => void;
}) {
  const icon = entry.isFolder
    ? "folder"
    : entry.kind === "spreadsheet"
      ? "sheet"
      : entry.kind === "deck"
        ? "deck"
        : "document";

  // A file Work Studio cannot open is shown but not offered, because hiding it would
  // misdescribe the folder.
  const openable = entry.isFolder || Boolean(entry.kind);

  return (
    <div
      role="row"
      tabIndex={openable ? 0 : -1}
      onClick={() => openable && onOpen()}
      onKeyDown={(event) => {
        if (event.key === "Enter" && openable) onOpen();
      }}
      style={{
        display: "grid",
        gridTemplateColumns: COLUMNS,
        padding: "10px 15px",
        borderBottom: last ? 0 : "1px solid #f2f0eb",
        fontSize: 12.5,
        alignItems: "center",
        cursor: openable ? "pointer" : "default",
        opacity: openable ? 1 : 0.65,
      }}
    >
      <div style={{ display: "flex", gap: 9, alignItems: "center", minWidth: 0 }}>
        <Icon name={icon} size={16} stroke="var(--muted)" width={1.8} />
        <div style={{ minWidth: 0 }}>
          <div
            style={{
              fontWeight: 560,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {entry.name}
          </div>
          {entry.isFolder && entry.count !== undefined ? (
            <div style={{ fontSize: 11, color: "var(--faint)", marginTop: 2 }}>
              {entry.count} {entry.count === 1 ? "thing" : "things"}
            </div>
          ) : null}
        </div>
      </div>
      <div style={{ color: "var(--muted)" }}>{whenChanged(entry.changed)}</div>
      <div style={{ color: "var(--muted)" }}>
        {entry.isFolder ? "—" : howBig(entry.size)}
      </div>
      <div />
    </div>
  );
}
