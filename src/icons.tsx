import { SVGProps } from "react";

/**
 * ClaudeBridge — FINAL icon set ("Crisp", Direction C, production)
 * Monochrome stroke icon set. Consistent 24×24 grid, stroke-width 1.7,
 * currentColor, round caps + joins. Each component spreads incoming props
 * (incl. className) onto the <svg> so callers can size/style freely.
 */

type P = SVGProps<SVGSVGElement> & { size?: number };

function Svg({ size = 16, children, ...rest }: P & { children: React.ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {children}
    </svg>
  );
}

/* Brand mark — a span/bridge arc linking two account anchors, with a
   conversation node lifted off-center at the keystone. The two feet are
   deliberately *different* (one solid node, one open ring) and asymmetric
   so the composition reads as "two distinct accounts being bridged"
   rather than a face. Kept 1:1 with the app tile mark (source.svg). */
export const IconLink = (p: P) => (
  <Svg {...p}>
    {/* flatter span so the apex doesn't read as a frown */}
    <path d="M5 16.5C5 11.8 8.1 9 12 9s7 2.8 7 7.5" />
    {/* piers — different lengths to break the symmetry */}
    <path d="M5 16.5v2.3" />
    <path d="M19 16.5v1.5" />
    {/* account A — solid foot node */}
    <circle cx="5" cy="19.6" r="1.45" fill="currentColor" stroke="none" />
    {/* account B — open ring foot node (distinct from A) */}
    <circle cx="19" cy="18.7" r="1.5" />
    {/* keystone conversation node, nudged off-center for asymmetry */}
    <path d="M13.4 8.2 14.2 5" />
    <circle cx="14.5" cy="3.9" r="1.55" fill="currentColor" stroke="none" />
  </Svg>
);

/* Refresh — re-scan. Single clean revolve with arrowhead. */
export const IconRefresh = (p: P) => (
  <Svg {...p}>
    <path d="M20 11a8 8 0 1 0-1.6 5.6" />
    <path d="M20 5v4h-4" />
  </Svg>
);

/* Search */
export const IconSearch = (p: P) => (
  <Svg {...p}>
    <circle cx="11" cy="11" r="6.5" />
    <path d="m20 20-3.7-3.7" />
  </Svg>
);

/* Terminal — used as the per-tool glyph. Prompt chevron + line. */
export const IconTerminal = (p: P) => (
  <Svg {...p}>
    <path d="m5 8 3.5 4L5 16" />
    <path d="M12 16h7" />
  </Svg>
);

/* CLI — bracketed command window for "命令行续聊". */
export const IconCli = (p: P) => (
  <Svg {...p}>
    <rect x="3" y="4.5" width="18" height="15" rx="2.5" />
    <path d="M3 8.5h18" />
    <path d="m7 12.5 2.2 2-2.2 2" />
    <path d="M12.5 16.5H17" />
  </Svg>
);

/* Messages — overlapping conversation bubbles (empty-state glyph). */
export const IconMessages = (p: P) => (
  <Svg {...p}>
    <path d="M8 13.5a2 2 0 0 1-2 2H5l-2 2v-9a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v1" />
    <path d="M11 16a2 2 0 0 0 2 2h6l2 2v-9a2 2 0 0 0-2-2h-6a2 2 0 0 0-2 2z" />
  </Svg>
);

/* Download — export to Markdown. */
export const IconDownload = (p: P) => (
  <Svg {...p}>
    <path d="M12 4v10" />
    <path d="m7.5 10.5 4.5 4 4.5-4" />
    <path d="M5 19.5h14" />
  </Svg>
);

/* Sliders — sort & filter controls. */
export const IconSliders = (p: P) => (
  <Svg {...p}>
    <path d="M4 6h9" />
    <path d="M17 6h3" />
    <circle cx="15" cy="6" r="2" />
    <path d="M4 12h3" />
    <path d="M11 12h9" />
    <circle cx="9" cy="12" r="2" />
    <path d="M4 18h9" />
    <path d="M17 18h3" />
    <circle cx="15" cy="18" r="2" />
  </Svg>
);

/* Check — selection. */
export const IconCheck = (p: P) => (
  <Svg {...p}>
    <path d="m5 12.5 4.5 4.5L19 6.5" />
  </Svg>
);

/* Theme: sun / moon / monitor (system). */
export const IconSun = (p: P) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  </Svg>
);
export const IconMoon = (p: P) => (
  <Svg {...p}>
    <path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z" />
  </Svg>
);
export const IconMonitor = (p: P) => (
  <Svg {...p}>
    <rect x="3" y="4" width="18" height="13" rx="2" />
    <path d="M8 21h8M12 17v4" />
  </Svg>
);
