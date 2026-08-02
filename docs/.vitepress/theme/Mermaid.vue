<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

<template>
  <figure class="mermaid-block" :class="{ 'is-open': open }">
    <button
      class="mermaid-toggle"
      type="button"
      :aria-expanded="open"
      :aria-controls="`${id}-panel`"
      @click="open = !open"
    >
      <span class="mermaid-caret" aria-hidden="true" />
      <span class="mermaid-kind">{{ kind }}</span>
      <span class="mermaid-action">{{ open ? "Collapse" : "Expand" }}</span>
    </button>

    <!--
      `v-show`, not `v-if`: the SVG stays in the document across a collapse, so
      re-expanding costs nothing. `v-if` would unmount it and force a redraw on
      every toggle of a diagram the reader is flipping back and forth.
    -->
    <div v-show="open" :id="`${id}-panel`" class="mermaid-panel">
      <p v-if="failed" class="mermaid-error">
        This diagram could not be rendered. Its source is in the page's
        Markdown.
      </p>
      <button
        v-else
        class="mermaid"
        type="button"
        :aria-label="`${kind}: enlarge`"
        @click="zoomed = true"
        v-html="svg"
      />
    </div>
  </figure>

  <!--
    Teleported to <body>: the diagram sits inside VitePress's content column,
    which establishes its own stacking and clipping context. Rendered in place,
    a full-viewport overlay is trapped by that column and by `overflow: hidden`
    on the block above.
  -->
  <Teleport to="body">
    <div
      v-if="zoomed"
      class="mermaid-zoom"
      role="dialog"
      aria-modal="true"
      :aria-label="`${kind}, enlarged`"
      @click="zoomed = false"
    >
      <div class="mermaid-zoom-figure" v-html="svg" />
      <button class="mermaid-zoom-close" type="button" @click="zoomed = false">
        Close
      </button>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
// Renders one ```mermaid fence: collapsible, and click-to-enlarge.
//
// Rendering happens in the browser because mermaid measures text to lay diagrams
// out, which needs a real DOM. `mermaid` is imported dynamically so it lands in
// its own chunk and is fetched only by pages that contain a diagram -- see
// ../mermaid.ts for what that is worth.
import { useData } from "vitepress";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { mermaidThemeVariables } from "./mermaid-theme";

const props = defineProps<{ id: string; graph: string }>();

// VitePress's own dark-mode ref, so diagrams re-render on the theme toggle
// instead of keeping colours from the mode they were first drawn in.
const { isDark } = useData();
const svg = ref("");

// Open by default: the diagrams ARE the content of these pages, and a reader who
// has to expand eight of them to read one page is worse off than one who
// scrolls. Collapsing is for getting a long page like ARCHITECTURE.md back under
// control once you know which diagram you want.
const open = ref(true);

// Click-to-enlarge, the interaction GitHub provides around its own mermaid
// output. Deliberately NOT medium-zoom, which the upstream this was ported from
// uses here: its `isSupported` is `node.tagName === "IMG"`, so handing it an
// <svg> binds nothing and every click is silently ignored -- while the
// `cursor: zoom-in` affordance promises otherwise. A diagram is not an <img>,
// so it gets an overlay of its own.
const zoomed = ref(false);

const source = computed(() => decodeURIComponent(props.graph));

// Labels the collapsed state, so a shut diagram still says what it is rather
// than reading as an anonymous grey bar. Mermaid's first keyword is the diagram
// type; anything unrecognised falls back to the generic word.
const KINDS: Record<string, string> = {
  classDiagram: "Class diagram",
  erDiagram: "Entity relationship diagram",
  flowchart: "Flowchart",
  gantt: "Gantt chart",
  gitGraph: "Git graph",
  graph: "Graph",
  journey: "User journey",
  mindmap: "Mindmap",
  pie: "Pie chart",
  quadrantChart: "Quadrant chart",
  sequenceDiagram: "Sequence diagram",
  stateDiagram: "State diagram",
  "stateDiagram-v2": "State diagram",
  timeline: "Timeline",
};

const kind = computed(() => {
  const first = source.value.trim().split(/\s+/)[0] ?? "";
  return KINDS[first] ?? "Diagram";
});

// `mermaid.render(id, ...)` stamps `id` onto the <svg> it returns, and removes
// any element already carrying that id before drawing. Passing the same id twice
// therefore deletes the diagram currently on the page, and the second render
// resolves against a node that is gone -- a collapse followed by an expand left
// an empty box, permanently. A fresh id per render avoids the collision;
// nothing reads these ids except mermaid itself.
let renderSeq = 0;

// Whether the drawn SVG still matches the current theme. Set when the site
// toggles dark mode while this diagram is collapsed, so it redraws when next
// opened rather than showing the previous mode's colours.
const stale = ref(false);
const hasRendered = ref(false);
const failed = ref(false);

const renderChart = async (): Promise<void> => {
  const mermaid = (await import("mermaid")).default;

  mermaid.initialize({
    startOnLoad: false,
    // Diagram labels in these docs contain markup such as `@actions/cache`; the
    // stricter levels would escape or drop it.
    securityLevel: "loose",
    // "base" is the only built-in theme that honours themeVariables wholesale,
    // which is what lets the palette follow the site instead of mermaid's stock
    // colours. Dark mode is a different set of variables, not a different theme.
    theme: "base",
    themeVariables: mermaidThemeVariables(isDark.value),
  });

  try {
    renderSeq += 1;
    const { svg: rendered } = await mermaid.render(
      `${props.id}-${renderSeq}`,
      source.value,
    );
    svg.value = rendered;
    failed.value = false;
    hasRendered.value = true;
    stale.value = false;
  } catch {
    // A diagram that will not parse must not take the page down with it, and an
    // empty bordered box gives a reader nothing to act on. The source is still
    // in the markdown, and `mise run mermaidlint` is the gate that should have
    // caught this before it shipped.
    failed.value = true;
    hasRendered.value = true;
  }
};

// Rendering is deferred until the diagram is first shown. It is open by default,
// so in practice that is on mount -- but nothing redraws a diagram nobody is
// looking at, and nothing redraws one that is already correct.
const renderIfNeeded = async (): Promise<void> => {
  if (!open.value) return;
  if (hasRendered.value && !stale.value) return;
  await renderChart();
};

// Escape closes the overlay, which a mouse-only close button does not give a
// keyboard user. Bound only while it is open, so the page carries no idle
// listener per diagram -- eight of them on ARCHITECTURE.md.
const onKeydown = (event: KeyboardEvent): void => {
  if (event.key === "Escape") zoomed.value = false;
};

watch(zoomed, (isZoomed) => {
  if (isZoomed) {
    window.addEventListener("keydown", onKeydown);
  } else {
    window.removeEventListener("keydown", onKeydown);
  }
  // The page behind must not scroll under the overlay.
  document.body.style.overflow = isZoomed ? "hidden" : "";
});

onMounted(renderIfNeeded);
watch(open, renderIfNeeded);
watch(isDark, async () => {
  if (open.value) {
    await renderChart();
    return;
  }
  // Collapsed: defer the redraw to whenever it is opened again.
  stale.value = true;
});
onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  document.body.style.overflow = "";
});
</script>

<style scoped>
.mermaid-block {
  margin: 20px 0;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  overflow: hidden;
}

.mermaid-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  border: 0;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-2);
  font-size: 13px;
  font-weight: 500;
  line-height: 1.4;
  text-align: left;
  cursor: pointer;
  transition:
    background-color 0.2s,
    color 0.2s;
}

.mermaid-toggle:hover {
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-1);
}

/* Keyboard focus must stay visible: these are the only controls on the block. */
.mermaid-toggle:focus-visible,
.mermaid:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: -2px;
}

/* A CSS triangle rather than an icon font or inline SVG, so the control adds
   no asset and inherits `currentColor` in both themes. */
.mermaid-caret {
  flex: none;
  width: 0;
  height: 0;
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
  border-left: 6px solid currentcolor;
  transition: transform 0.2s;
}

.is-open .mermaid-caret {
  transform: rotate(90deg);
}

.mermaid-kind {
  flex: 1 1 auto;
}

.mermaid-action {
  flex: none;
  font-size: 12px;
  opacity: 0.7;
}

.mermaid-panel {
  padding: 16px 0;
}

/* A <button>, not a <div>: enlarging is a real action, so it belongs on
   something focusable and keyboard-activatable rather than a click handler
   bolted to a container. Stripped back to look like the container it replaces. */
.mermaid {
  display: block;
  width: 100%;
  padding: 0;
  border: 0;
  background: none;
  overflow-x: auto;
  text-align: center;
  cursor: zoom-in;
  font: inherit;
  color: inherit;
}

.mermaid-error {
  margin: 0;
  padding: 0 12px;
  color: var(--vp-c-danger-1, var(--vp-c-text-2));
  font-size: 14px;
}

.mermaid-zoom {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  background: var(--vp-c-bg);
  cursor: zoom-out;
  overflow: auto;
}

.mermaid-zoom-figure {
  width: 100%;
  max-width: 1600px;
}

.mermaid-zoom-figure :deep(svg) {
  width: 100%;
  height: auto;
  max-height: none;
}

.mermaid-zoom-close {
  position: fixed;
  top: 16px;
  right: 16px;
  padding: 6px 14px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1);
  font-size: 13px;
  cursor: pointer;
}

.mermaid-zoom-close:hover {
  background: var(--vp-c-default-soft);
}

@media (prefers-reduced-motion: reduce) {
  .mermaid-toggle,
  .mermaid-caret {
    transition: none;
  }
}
</style>
