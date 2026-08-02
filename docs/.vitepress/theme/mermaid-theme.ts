// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Mermaid theme variables derived from the site's own CSS custom properties.
//
// Mermaid cannot read CSS variables: it does colour arithmetic (lighten, darken,
// contrast) on the values it is given, so it needs resolved colours. Everything
// here is therefore read off <html> at render time and handed over as hex. The
// upside is that diagrams follow the VitePress palette -- including the dark-mode
// toggle -- rather than carrying a second, hand-maintained set of brand colours.

/** Reads a CSS custom property off <html>, falling back when it is not set. */
const cssVar = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return value || fallback;
};

/** Hue (0-360) of a #rgb / #rrggbb colour, or `fallback` if it is not one. */
const hueOf = (color: string, fallback: number): number => {
  const hex = color.replace("#", "");
  const full = hex.length === 3 ? [...hex].map((c) => c + c).join("") : hex;
  if (!/^[0-9a-f]{6}$/i.test(full)) return fallback;

  // Destructured explicitly rather than from `.map`, which types every element
  // as `number | undefined` under `noUncheckedIndexedAccess`. The regex above
  // has already proved all six digits are present, so the indices are safe --
  // but the compiler cannot see that through an array.
  const channel = (i: number): number =>
    parseInt(full.slice(i, i + 2), 16) / 255;
  const r = channel(0);
  const g = channel(2);
  const b = channel(4);
  const max = Math.max(r, g, b);
  const span = max - Math.min(r, g, b);
  if (span === 0) return fallback;

  const sextant =
    max === r
      ? ((g - b) / span) % 6
      : max === g
        ? (b - r) / span + 2
        : (r - g) / span + 4;
  return (sextant * 60 + 360) % 360;
};

/** Builds a #rrggbb string from HSL, so mermaid only ever receives hex. */
const hsl = (h: number, s: number, l: number): string => {
  const chroma = (s / 100) * Math.min(l / 100, 1 - l / 100);
  const channel = (n: number): string => {
    const k = (n + h / 30) % 12;
    const value = l / 100 - chroma * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(255 * value)
      .toString(16)
      .padStart(2, "0");
  };
  return `#${channel(0)}${channel(8)}${channel(4)}`;
};

// Mindmap branches, pie slices and similar "one colour per section" diagrams read
// cScale0..11. Left alone, mermaid hands out its stock yellow/green/pink rainbow,
// which belongs to no palette in particular.
//
// These offsets stay within +/-40 deg of the brand hue. Wider spreads were tried
// first and drift out of the family entirely -- at +/-80 an indigo brand yields
// mint and pink, which is a different rainbow rather than the site's palette.
// Within the narrower band, hues alone are too close to separate adjacent
// branches, so lightness is nudged per section as a second axis. Ordered so the
// first few sections -- all most diagrams use -- are the furthest apart.
const SECTION_HUE_OFFSETS = [
  0, 34, -30, 18, -40, 26, -14, 40, -22, 10, -36, 30,
];
const SECTION_LIGHTNESS_STEPS = [0, -3, 3, -5, 2, -2, 5, -4, 1, -1, 4, -3];

/** Per-mode tuning for section fills; dark mode needs darker, calmer blocks. */
const SECTION_TONE = {
  light: {
    saturation: 44,
    lightness: 87,
    border: { saturation: 40, lightness: 62 },
  },
  dark: {
    saturation: 32,
    lightness: 28,
    border: { saturation: 36, lightness: 52 },
  },
};

/**
 * Theme variables for mermaid's `base` theme, matching the current VitePress mode.
 *
 * Must run in the browser: it reads computed styles off the document.
 */
export const mermaidThemeVariables = (
  isDark: boolean,
): Record<string, string> => {
  const brand = cssVar("--vp-c-brand-1", isDark ? "#a8b1ff" : "#3451b2");
  const text = cssVar("--vp-c-text-1", isDark ? "#dfdfd6" : "#3c3c43");
  const mutedText = cssVar("--vp-c-text-2", isDark ? "#98989f" : "#67676c");
  const background = cssVar("--vp-c-bg", isDark ? "#1b1b1f" : "#ffffff");
  const surface = cssVar("--vp-c-bg-soft", isDark ? "#202127" : "#f6f6f7");
  const divider = cssVar("--vp-c-divider", isDark ? "#2e2e32" : "#e2e2e3");
  const border = cssVar("--vp-c-border", isDark ? "#3c3f44" : "#c2c2c4");

  const hue = hueOf(brand, isDark ? 231 : 227);
  const tone = isDark ? SECTION_TONE.dark : SECTION_TONE.light;

  const sections = Object.fromEntries(
    SECTION_HUE_OFFSETS.flatMap((offset, index) => {
      // `?? 0`: the two arrays are the same length by construction, but indexing
      // one by the other's index is unprovable to the compiler. Falling back to
      // no adjustment is the harmless answer if they ever diverge.
      const lightness = tone.lightness + (SECTION_LIGHTNESS_STEPS[index] ?? 0);
      return [
        [`cScale${index}`, hsl(hue + offset, tone.saturation, lightness)],
        [
          `cScaleBorder${index}`,
          hsl(hue + offset, tone.border.saturation, tone.border.lightness),
        ],
        [`cScaleLabel${index}`, text],
      ];
    }),
  );

  // Node fill and stroke, shared by the flowchart-style variables below. Kept a
  // shade off the page background so boxes read as boxes, and not so pale that
  // the mindmap's root circle disappears into the page.
  const nodeFill = hsl(hue, isDark ? 20 : 44, isDark ? 23 : 90);
  const nodeStroke = hsl(hue, isDark ? 30 : 40, isDark ? 46 : 70);

  return {
    ...sections,
    darkMode: String(isDark),
    background,
    // Deliberately no fontFamily or fontSize override. Mermaid sizes each node
    // from its own measurement of the label, and that measurement does not track
    // an overridden font: both the site's webfont stack and a plain system stack
    // were tried, and both produced boxes that clip their own text (a third line
    // appearing where two were budgeted for). Awaiting document.fonts.ready is
    // not enough -- the mismatch is in the sizing, not a font-loading race. Colour
    // is what this file is for; typography stays mermaid's.

    // A mindmap's root node is filled from `git0`, not from the cScale series --
    // mermaid reuses a gitGraph variable for it. Left unset it comes out as the
    // page background, i.e. an invisible circle. Deeper than the branch fills, so
    // the centre of the diagram reads as the centre.
    git0: hsl(hue, isDark ? 34 : 40, isDark ? 38 : 80),
    gitBranchLabel0: text,

    // Boxes: a brand-tinted surface rather than mermaid's stock lavender.
    primaryColor: nodeFill,
    primaryBorderColor: nodeStroke,
    primaryTextColor: text,
    secondaryColor: surface,
    secondaryBorderColor: border,
    secondaryTextColor: text,
    tertiaryColor: background,
    tertiaryBorderColor: divider,
    tertiaryTextColor: mutedText,

    // Edges and labels sit on the page, so they follow the text colours.
    lineColor: isDark ? mutedText : border,
    textColor: text,
    titleColor: text,
    edgeLabelBackground: background,
    nodeBorder: nodeStroke,
    mainBkg: nodeFill,

    // Subgraph containers: quieter than the nodes they hold.
    clusterBkg: isDark ? background : surface,
    clusterBorder: divider,

    noteBkgColor: surface,
    noteTextColor: text,
    noteBorderColor: border,
  };
};
