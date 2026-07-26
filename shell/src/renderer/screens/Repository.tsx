/**
 * Documents.
 *
 * A view over the real folder, never a second store. Folders shown are folders on
 * disk; kinds are filters over them rather than folders of their own, because an
 * app-only taxonomy vanishes the moment the User opens Finder.
 *
 * `Used in` is the second axis: one piece of work produces several files and one file
 * is used by several pieces of work, so the derivation lineage is navigable rather
 * than merely described.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { FILES, type FileRow } from "../fixtures.ts";
import { Button, Chip, Field, Icon } from "../components/primitives.tsx";
import { Empty } from "../components/states.tsx";
import type { Route } from "../routes.ts";

const KINDS = [
  { key: "everything", label: t("repo.kind.everything") },
  { key: "document", label: t("repo.kind.documents") },
  { key: "deck", label: t("repo.kind.decks") },
  { key: "sheet", label: t("repo.kind.spreadsheets") },
  { key: "pdf", label: t("repo.kind.pdfs") },
] as const;

const COLUMNS = "2.1fr 1.2fr 1.4fr 78px";

const ROUTE_KIND: Partial<Record<Route, string>> = {
  documents: "document",
  decks: "deck",
  spreadsheets: "sheet",
};

export function Repository({
  onNavigate,
  route = "repository",
  onOpenFile,
}: {
  onNavigate: (route: Route) => void;
  route?: Route;
  onOpenFile?: () => void;
}) {
  const [kind, setKind] = useState<string>(ROUTE_KIND[route] ?? "everything");

  // Folders always show, because they are how the User navigates. That means a kind
  // filter can produce a list of folders and no files, which reads as a result when it
  // is not one — so the count of matching files is tracked separately and said plainly.
  const rows = FILES.filter(
    (file) => kind === "everything" || file.kind === "folder" || file.kind === kind,
  );
  const matchingFiles = rows.filter((file) => file.kind !== "folder").length;
  const filtered = kind !== "everything";

  return (
    <main className="main">
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
        <h1 className="h1" style={{ margin: 0 }}>
          {KINDS.find((option) => option.key === kind)?.key === "everything"
            ? t("repo.all_files")
            : KINDS.find((option) => option.key === kind)!.label}
        </h1>
        <div className="ml" style={{ display: "flex", gap: 7 }}>
          {onOpenFile && (
            <Button small onClick={onOpenFile}>
              {t("new.open_a_file")}
            </Button>
          )}
          <Button small>{t("repo.new_folder")}</Button>
          <Button small>{t("repo.show_in_finder")}</Button>
        </div>
      </div>
      <p style={{ fontSize: 12, color: "var(--muted)", margin: "0 0 12px" }}>
        <b style={{ color: "#26241f" }}>Documents › Work Studio</b> — 14 files, 2 folders on your Mac
      </p>

      <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 14 }}>
        {KINDS.map((option) => (
          <Chip key={option.key} on={kind === option.key} onClick={() => setKind(option.key)}>
            {option.label}
          </Chip>
        ))}
        <Field
          placeholder={t("repo.search")}
          style={{ marginLeft: "auto", width: 150, padding: "5px 10px", fontSize: 12 }}
        />
      </div>

      {rows.length === 0 || (filtered && matchingFiles === 0) ? (
        <Empty
          icon="folder"
          title="Nothing of that kind here"
          what="Files you and I make together live in Documents › Work Studio, as ordinary files you can open anywhere. Folders below may still hold some."
        />
      ) : null}

      <div
        style={{
          background: "var(--card)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius)",
          overflow: "hidden",
          display: rows.length === 0 ? "none" : "block",
          marginTop: filtered && matchingFiles === 0 ? 12 : 0,
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
          <div>{t("repo.col.used_in")}</div>
          <div>{t("repo.col.versions")}</div>
        </div>
        {rows.map((file, index) => (
          <Row key={file.id} file={file} last={index === rows.length - 1} onNavigate={onNavigate} />
        ))}
      </div>

      <p className="hint" style={{ marginTop: 14 }}>
        {t("repo.real_folders")}
      </p>
    </main>
  );
}

function Row({
  file,
  last,
  onNavigate,
}: {
  file: FileRow;
  last: boolean;
  onNavigate: (route: Route) => void;
}) {
  const icon =
    file.kind === "folder"
      ? "folder"
      : file.kind === "sheet"
        ? "sheet"
        : file.kind === "deck"
          ? "deck"
          : "document";

  const open = () => {
    if (file.kind === "sheet") onNavigate("spreadsheetWorkspace");
    else if (file.kind === "deck") onNavigate("deckWorkspace");
    else if (file.kind === "document") onNavigate("documentWorkspace");
  };

  return (
    <div
      role="row"
      tabIndex={0}
      onClick={open}
      onKeyDown={(event) => {
        if (event.key === "Enter") open();
      }}
      style={{
        display: "grid",
        gridTemplateColumns: COLUMNS,
        padding: "10px 15px",
        borderBottom: last ? 0 : "1px solid #f2f0eb",
        fontSize: 12.5,
        alignItems: "center",
        cursor: file.kind === "folder" ? "default" : "pointer",
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
            {file.name}
          </div>
          {file.sub ? (
            <div style={{ fontSize: 11, color: "var(--faint)", marginTop: 2 }}>{file.sub}</div>
          ) : null}
        </div>
      </div>
      <div style={{ color: "var(--muted)" }}>
        {file.changed}
        {file.changedBy ? (
          <div style={{ fontSize: 11, color: "var(--faint)" }}>{file.changedBy}</div>
        ) : null}
      </div>
      <div style={{ color: "var(--muted)" }}>
        {file.usedIn.length === 0 ? (
          "—"
        ) : (
          file.usedIn.map((thread) => (
            <div
              key={thread}
              style={{
                textDecoration: "underline",
                textDecorationColor: "#d5d0c7",
                textUnderlineOffset: 2,
                color: "var(--ink-soft)",
                fontSize: 12,
              }}
            >
              {thread}
            </div>
          ))
        )}
      </div>
      <div style={{ color: "var(--muted)" }}>{file.versions ?? "—"}</div>
    </div>
  );
}
