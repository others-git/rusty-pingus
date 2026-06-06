// Theme switcher — vanilla, page-agnostic. Injects a discreet edge handle that
// opens a dropdown of themes; persists the choice in localStorage. The actual
// theming is done by themes.css via the `data-theme` attribute on <html>.
(function () {
  const KEY = 'theme';
  // Cyberpunk is the default; there is no plain "Default" theme.
  const DEFAULT_THEME = 'cyber';
  const THEMES = [
    { id: 'cyber',   name: '⚡ Cyberpunk — NEON CITY' },
    { id: 'steam',   name: '⚙ Steampunk — BRASS & STEAM' },
    { id: 'solar',   name: '🌻 Solarpunk — SUNFLOWER' },
    { id: 'vapor',   name: '🌴 Vaporwave — A E S T H E T I C' },
    { id: 'chaos',   name: '🌀 CHAOS — ENTROPY' },
  ];

  // The CHAOS theme references an SVG turbulence/displacement filter (#chaos-warp)
  // to warp elements. CSS can't define SVG filters, so inject it once (harmless
  // for other themes; it has no visual effect unless referenced).
  function injectChaosFilter() {
    if (document.getElementById('chaos-warp-svg')) return;
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.id = 'chaos-warp-svg';
    svg.setAttribute('width', '0');
    svg.setAttribute('height', '0');
    svg.style.cssText = 'position:absolute;width:0;height:0';
    svg.innerHTML =
      '<filter id="chaos-warp">' +
      '<feTurbulence type="fractalNoise" baseFrequency="0.012 0.016" numOctaves="2" result="n">' +
      '<animate attributeName="baseFrequency" dur="16s" values="0.010 0.014;0.022 0.009;0.010 0.014" repeatCount="indefinite"/>' +
      '</feTurbulence>' +
      '<feDisplacementMap in="SourceGraphic" in2="n" scale="12"/>' +
      '</filter>';
    document.body.appendChild(svg);
  }

  // CHAOS background: a field of randomized geometric shapes. CSS can't generate
  // randomness, so we seed positions/sizes/shapes/parallax-depth here once; the
  // *reaction* to the mouse is pure CSS (shapes translate/rotate via --mx/--my).
  function injectChaosShapes() {
    if (document.getElementById('chaos-shapes')) return;
    const clips = [
      'polygon(50% 0,0 100%,100% 100%)',                                   // triangle
      'polygon(50% 0,100% 50%,50% 100%,0 50%)',                            // diamond
      'polygon(25% 0,75% 0,100% 50%,75% 100%,25% 100%,0 50%)',             // hexagon
      'polygon(0 0,50% 22%,100% 0,100% 100%,50% 78%,0 100%)',              // hourglass-ish
      'none',                                                              // square
      'polygon(50% 0,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)', // star
    ];
    const colors = ['#9dff00', '#ff0044', '#00e5ff', '#e8e8ff'];
    const rnd = (a, b) => a + Math.random() * (b - a);
    let html = '';
    for (let i = 0; i < 18; i++) {
      const size = rnd(36, 200) | 0;
      const clip = clips[(Math.random() * clips.length) | 0];
      const c = colors[(Math.random() * colors.length) | 0];
      const px = (rnd(18, 80) * (Math.random() < 0.5 ? -1 : 1)) | 0;
      const py = (rnd(18, 80) * (Math.random() < 0.5 ? -1 : 1)) | 0;
      const style = [
        `top:${rnd(-5, 100).toFixed(1)}%`, `left:${rnd(-5, 100).toFixed(1)}%`,
        `width:${size}px`, `height:${size}px`,
        `--px:${px}px`, `--py:${py}px`,
        `--spin:${rnd(9, 34).toFixed(1)}s`,
        `animation-delay:${(-rnd(0, 30)).toFixed(1)}s`,
        Math.random() < 0.5 ? 'animation-direction:reverse' : '',
        clip !== 'none' ? `clip-path:${clip}` : '',
        `background:${c}${Math.random() < 0.3 ? '33' : '14'}`,
        `box-shadow:0 0 0 1px ${c}66`,
        `filter:drop-shadow(0 0 7px ${c}aa)`,
      ].filter(Boolean).join(';');
      html += `<i class="chaos-shape" style="${style}"></i>`;
    }
    const wrap = document.createElement('div');
    wrap.id = 'chaos-shapes';
    wrap.setAttribute('aria-hidden', 'true');
    wrap.innerHTML = html;
    document.body.appendChild(wrap);
  }

  // Resolve a stored value to a real theme: nothing stored, or the removed legacy
  // "default", both map to the cyberpunk default.
  const normalize = (id) => (!id || id === 'default') ? DEFAULT_THEME : id;
  const current = () => { try { return normalize(localStorage.getItem(KEY)); } catch (e) { return DEFAULT_THEME; } };
  const save = (id) => { try { localStorage.setItem(KEY, id); } catch (e) {} };
  function apply(id) {
    // Every theme (including the cyberpunk default) is applied via the attribute.
    document.documentElement.setAttribute('data-theme', normalize(id));
  }

  function build() {
    if (document.getElementById('theme-switcher')) return;

    const root = document.createElement('div');
    root.id = 'theme-switcher';

    const handle = document.createElement('button');
    handle.id = 'theme-handle';
    handle.type = 'button';
    handle.title = 'Change theme';
    handle.setAttribute('aria-label', 'Change theme');
    handle.innerHTML = '<span class="ts-emoji">🎨</span>';

    const menu = document.createElement('div');
    menu.id = 'theme-menu';
    menu.hidden = true;

    const mark = () => {
      const cur = current();
      menu.querySelectorAll('.ts-item').forEach((el) =>
        el.classList.toggle('ts-active', el.dataset.theme === cur));
    };
    const open = () => { menu.hidden = false; root.classList.add('ts-open'); mark(); };
    const close = () => { menu.hidden = true; root.classList.remove('ts-open'); };

    const title = document.createElement('div');
    title.className = 'ts-title';
    title.textContent = 'Theme';
    menu.appendChild(title);

    THEMES.forEach((t) => {
      const item = document.createElement('button');
      item.type = 'button';
      item.className = 'ts-item';
      item.dataset.theme = t.id;
      item.textContent = t.name;
      item.addEventListener('click', () => { apply(t.id); save(t.id); mark(); close(); });
      menu.appendChild(item);
    });

    handle.addEventListener('click', (e) => { e.stopPropagation(); menu.hidden ? open() : close(); });
    document.addEventListener('click', (e) => { if (!root.contains(e.target)) close(); });
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(); });

    root.appendChild(handle);
    root.appendChild(menu);
    document.body.appendChild(root);
    injectChaosFilter();
    injectChaosShapes();
    mark();
  }

  apply(current()); // safety net in case the inline <head> snippet didn't run
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();

  // Expose the cursor position to CSS as --mx/--my (0..1) and --mxpx/--mypx (%),
  // so themes can do background interactions (spotlights, parallax). rAF-throttled.
  (function trackPointer() {
    let x = 0.5, y = 0.5, queued = false;
    const root = document.documentElement.style;
    root.setProperty('--mx', '0.5'); root.setProperty('--my', '0.5');
    root.setProperty('--mxpx', '50%'); root.setProperty('--mypx', '50%');
    addEventListener('pointermove', (e) => {
      x = e.clientX / Math.max(1, innerWidth);
      y = e.clientY / Math.max(1, innerHeight);
      if (queued) return;
      queued = true;
      requestAnimationFrame(() => {
        queued = false;
        root.setProperty('--mx', x.toFixed(4));
        root.setProperty('--my', y.toFixed(4));
        root.setProperty('--mxpx', (x * 100).toFixed(2) + '%');
        root.setProperty('--mypx', (y * 100).toFixed(2) + '%');
      });
    }, { passive: true });
  })();
})();
