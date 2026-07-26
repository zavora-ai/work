/**
 * A chart the file holds.
 *
 * Drawn from numbers the Core already resolved, so a chart and the cells beside it cannot
 * disagree: this component never reads a range and never works out what a reference means. It
 * receives points and plots them.
 *
 * A kind it has no way to draw says so rather than being drawn as something else, because a
 * radar chart shown as bars is a lie about the User's own file.
 */

import { columnName, type Chart, type ChartSeries } from "../../shared/grid.ts";

/// Enough colours for the series a person puts on one chart, and distinguishable from each other
/// at the size a chart is drawn here.
const COLOURS = ["#4a6d8c", "#8c6f4a", "#5f8c4a", "#8c4a6d", "#4a8c8a", "#6d4a8c"];

export function ChartView({ chart, width = 460 }: { chart: Chart; width?: number }) {
  const height = Math.round(width * 0.6);

  return (
    <figure
      style={{
        margin: 0,
        background: "#fff",
        border: "1px solid var(--border)",
        borderRadius: 10,
        padding: "12px 14px",
        width,
      }}
    >
      <figcaption
        style={{
          fontSize: 12.5,
          fontWeight: 650,
          marginBottom: 8,
          color: "#33302a",
        }}
      >
        {chart.title ?? "Chart"}
        {/* Where the file puts it. Charts are shown in a strip under the grid rather than
            floating over the cells, so saying the cell is how the User connects the two. */}
        <span style={{ fontWeight: 400, color: "var(--faint)", marginLeft: 6, fontSize: 11 }}>
          {/* The separator is in the text, not only in the margin: read aloud, "Chartat E5"
              is what a missing space sounds like. */}
          {` · at ${columnName(chart.atCol)}${chart.atRow + 1}`}
        </span>
      </figcaption>
      {chart.kind === "other" ? (
        <p className="hint" style={{ margin: 0, fontSize: 12 }}>
          This chart is in your file and Work Studio cannot draw this kind yet. Nothing has been
          changed.
        </p>
      ) : (
        <Plot chart={chart} width={width - 28} height={height} />
      )}
      {chart.series.length > 1 ? (
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginTop: 8 }}>
          {chart.series.map((series, index) => (
            <span
              key={series.name ?? index}
              style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11 }}
            >
              <span
                aria-hidden="true"
                style={{
                  width: 9,
                  height: 9,
                  borderRadius: 2,
                  background: COLOURS[index % COLOURS.length],
                }}
              />
              {series.name ?? `Series ${index + 1}`}
            </span>
          ))}
        </div>
      ) : null}
    </figure>
  );
}

/** Every number in the chart, for working out the scale. */
function allValues(series: ChartSeries[]): number[] {
  return series.flatMap((one) => one.values.filter((value): value is number => value !== null));
}

function Plot({ chart, width, height }: { chart: Chart; width: number; height: number }) {
  const numbers = allValues(chart.series);
  if (numbers.length === 0) {
    return (
      <p className="hint" style={{ margin: 0, fontSize: 12 }}>
        There are no numbers in this chart's range.
      </p>
    );
  }

  if (chart.kind === "pie") return <Pie chart={chart} width={width} height={height} />;

  const top = Math.max(...numbers, 0);
  const bottom = Math.min(...numbers, 0);
  const span = top - bottom || 1;

  const padLeft = 46;
  const padBottom = 22;
  const plotWidth = width - padLeft - 6;
  const plotHeight = height - padBottom - 8;
  const labels = chart.series[0]?.labels ?? [];
  const points = Math.max(...chart.series.map((one) => one.values.length), 1);

  const y = (value: number) => 8 + plotHeight - ((value - bottom) / span) * plotHeight;
  const xStep = plotWidth / points;

  const horizontal = chart.kind === "bar";

  return (
    <svg
      width={width}
      height={height}
      role="img"
      // The whole chart in one sentence, because a picture of numbers is unreadable to anyone
      // using a screen reader unless it says what it shows.
      aria-label={`${chart.title ?? "Chart"}: ${chart.series
        .map(
          (one, index) =>
            `${one.name ?? `series ${index + 1}`} ${one.values
              .map((value, at) => `${labels[at] ?? at + 1} ${value ?? "no value"}`)
              .join(", ")}`,
        )
        .join("; ")}`}
    >
      {/* The zero line, where the numbers cross it. Without it a chart of positives and
          negatives reads as though everything were positive. */}
      <line
        x1={padLeft}
        x2={width - 6}
        y1={y(0)}
        y2={y(0)}
        stroke="#d5d0c7"
        strokeWidth={1}
      />
      <text x={2} y={y(top) + 4} fontSize={9.5} fill="#8a8580">
        {short(top)}
      </text>
      <text x={2} y={y(bottom) + 4} fontSize={9.5} fill="#8a8580">
        {short(bottom)}
      </text>

      {chart.series.map((series, seriesIndex) =>
        series.values.map((value, at) => {
          if (value === null) return null;
          const colour = COLOURS[seriesIndex % COLOURS.length];
          const slot = xStep / Math.max(chart.series.length, 1);
          const left = padLeft + at * xStep + seriesIndex * slot + 2;

          if (chart.kind === "line" || chart.kind === "area" || chart.kind === "scatter") {
            return (
              <circle
                key={`${seriesIndex}-${at}`}
                cx={padLeft + at * xStep + xStep / 2}
                cy={y(value)}
                r={2.6}
                fill={colour}
              />
            );
          }

          if (horizontal) {
            const length = ((value - bottom) / span) * plotWidth;
            const barHeight = Math.max(plotHeight / points / chart.series.length - 3, 2);
            return (
              <rect
                key={`${seriesIndex}-${at}`}
                x={padLeft}
                y={8 + at * (plotHeight / points) + seriesIndex * barHeight}
                width={Math.max(length, 1)}
                height={barHeight}
                fill={colour}
              />
            );
          }

          const zero = y(0);
          const here = y(value);
          return (
            <rect
              key={`${seriesIndex}-${at}`}
              x={left}
              y={Math.min(zero, here)}
              width={Math.max(slot - 4, 2)}
              height={Math.max(Math.abs(zero - here), 1)}
              fill={colour}
            />
          );
        }),
      )}

      {/* Joining the dots, for the kinds where the line is the point. */}
      {chart.kind === "line" || chart.kind === "area"
        ? chart.series.map((series, seriesIndex) => (
            <polyline
              key={`line-${seriesIndex}`}
              fill="none"
              stroke={COLOURS[seriesIndex % COLOURS.length]}
              strokeWidth={1.6}
              points={series.values
                .map((value, at) =>
                  value === null
                    ? null
                    : `${padLeft + at * xStep + xStep / 2},${y(value)}`,
                )
                .filter(Boolean)
                .join(" ")}
            />
          ))
        : null}

      {labels.map((label, at) =>
        label && at % Math.ceil(points / 8 || 1) === 0 ? (
          <text
            key={`label-${at}`}
            x={padLeft + at * xStep + xStep / 2}
            y={height - 6}
            fontSize={9.5}
            fill="#8a8580"
            textAnchor="middle"
          >
            {label.length > 9 ? `${label.slice(0, 8)}…` : label}
          </text>
        ) : null,
      )}
    </svg>
  );
}

function Pie({ chart, width, height }: { chart: Chart; width: number; height: number }) {
  const series = chart.series[0];
  const values = (series?.values ?? []).map((value) => value ?? 0);
  const total = values.reduce((sum, value) => sum + Math.abs(value), 0);
  if (total === 0) {
    return (
      <p className="hint" style={{ margin: 0, fontSize: 12 }}>
        There is nothing to divide up in this chart's range.
      </p>
    );
  }

  const radius = Math.min(width, height) / 2 - 8;
  const centre = { x: width / 2, y: height / 2 };
  let sweptSoFar = -Math.PI / 2;

  return (
    <svg
      width={width}
      height={height}
      role="img"
      aria-label={`${chart.title ?? "Chart"}: ${values
        .map((value, at) => `${series?.labels[at] ?? at + 1} ${value}`)
        .join(", ")}`}
    >
      {values.map((value, at) => {
        const share = Math.abs(value) / total;
        const from = sweptSoFar;
        const to = from + share * Math.PI * 2;
        sweptSoFar = to;
        const large = to - from > Math.PI ? 1 : 0;
        const path = [
          `M ${centre.x} ${centre.y}`,
          `L ${centre.x + radius * Math.cos(from)} ${centre.y + radius * Math.sin(from)}`,
          `A ${radius} ${radius} 0 ${large} 1 ${centre.x + radius * Math.cos(to)} ${
            centre.y + radius * Math.sin(to)
          }`,
          "Z",
        ].join(" ");
        return <path key={at} d={path} fill={COLOURS[at % COLOURS.length]} />;
      })}
    </svg>
  );
}

/** A number short enough for an axis. */
function short(value: number): string {
  const size = Math.abs(value);
  if (size >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}m`;
  if (size >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return `${Math.round(value)}`;
}
