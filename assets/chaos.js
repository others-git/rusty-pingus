/* chaos.js — "The Vast Dark" void-noise Houdini Paint Worklet
 *
 * Only activates when data-theme="chaos". Registers a painter that draws
 * sparse deep-sea bioluminescent grain over body::after — dim purples and
 * rare sickly greens, re-rendered every frame because --void-breath is a
 * registered @property that animates.
 *
 * Cleans up when the theme changes away.
 */
(function () {
  'use strict';

  const ROOT = document.documentElement;
  function isChaos() { return ROOT.getAttribute('data-theme') === 'chaos'; }

  /* ── Worklet source ─────────────────────────────────────────────────────── */
  // Reads --void-breath (0–2π, <number>) so the worklet re-paints every
  // animation frame, keeping the grain alive rather than static.
  const WORKLET_SRC = /* js */ `
    registerPaint('void-noise', class {
      static get inputProperties() { return ['--void-breath']; }
      static get contextOptions()  { return { alpha: true }; }

      paint(ctx, { width, height }, props) {
        // Sparse pixels — like phosphorescent plankton in black water
        const count = Math.ceil(width * height * 0.0015);
        for (let i = 0; i < count; i++) {
          const x = Math.random() * width;
          const y = Math.random() * height;

          // 80% deep purple/indigo, 20% sickly teal — the colors of things
          // that live where no light reaches
          const isTeal = Math.random() < 0.20;
          const h = isTeal
            ? 152 + (Math.random() * 28 - 14) | 0
            : 268 + (Math.random() * 44 - 22) | 0;
          const s = (38 + Math.random() * 22) | 0;
          const l = (10 + Math.random() * 16) | 0;
          const a = (Math.random() * 0.14 + 0.02).toFixed(3);

          ctx.fillStyle = \`hsla(\${h},\${s}%,\${l}%,\${a})\`;
          // Rare slightly-larger sparks
          const sz = Math.random() < 0.03 ? 2 : 1;
          ctx.fillRect(x | 0, y | 0, sz, sz);
        }
      }
    });
  `;

  /* ── Lifecycle ──────────────────────────────────────────────────────────── */

  let registered = false;

  function registerWorklet() {
    if (!('paintWorklet' in CSS) || registered) return;
    registered = true;
    const blob = new Blob([WORKLET_SRC], { type: 'text/javascript' });
    CSS.paintWorklet.addModule(URL.createObjectURL(blob))
      .then(applyPaint)
      .catch(() => { /* silently degrade — spotlight still shows */ });
  }

  function applyPaint() {
    if (document.getElementById('void-paint-style')) return;
    const s = document.createElement('style');
    s.id = 'void-paint-style';
    // Layer the noise UNDER the cursor spotlight; both use screen blend
    s.textContent = `
      html[data-theme="chaos"] body::after {
        background:
          paint(void-noise),
          radial-gradient(280px circle at var(--mxpx) var(--mypx), rgba(28,5,52,0.45), transparent 72%);
        mix-blend-mode: screen;
      }
    `;
    document.head.appendChild(s);
  }

  function removePaint() {
    document.getElementById('void-paint-style')?.remove();
  }

  function activate() {
    registerWorklet();
    // If the worklet was already registered (theme toggled back), re-inject
    if (registered) applyPaint();
  }

  // Observe theme attribute — clean up immediately on theme switch
  new MutationObserver(() => {
    if (isChaos()) activate(); else removePaint();
  }).observe(ROOT, { attributes: true, attributeFilter: ['data-theme'] });

  // Initial run (handles both deferred-load and already-loaded cases)
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => { if (isChaos()) activate(); });
  } else {
    if (isChaos()) activate();
  }
})();
