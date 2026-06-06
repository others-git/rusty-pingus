function monitorDetail() {
  const CIRCUMFERENCE = 2 * Math.PI * 40; // r=40 → ≈251.2
  const WINDOW_SECS = { '1h': 3600, '24h': 86400, '7d': 604800, '30d': 2592000 };
  const RAW_LIMIT = 1000;   // max raw probes to render individually
  const BUCKET_TARGET = 300;
  const REFETCH_DEBOUNCE_MS = 300;
  const SUSTAINED_MS = 60_000; // outages >= this are drawn as a fat labeled band

  // ── Non-reactive state (closure) ──────────────────────────────────────────
  // The ECharts instance and chart data are kept OUT of Alpine's reactive object:
  // a chart instance is full of circular refs, and proxying it makes the library
  // recurse over reactive get-traps until the stack overflows.
  let chart = null;
  let lineData = [];        // [[tsMs, y|null], ...]
  let downIntervals = [];   // [[startMs, endMs], ...]
  let tlSegments = [];      // state timeline: [{ state, label, start, end, color }, ...]
  const tlColorMap = new Map(); // stable value → color across re-renders/zoom (publicip)
  let windowFrom = null, windowTo = null; // ms; the fixed axis extent (active window)
  let refetchSeq = 0;
  let debounceTimer = null;
  let isApplying = false;   // guard: programmatic setOption/dispatch must not refetch
  let resizeBound = false;

  return {
    monitorName: decodeURIComponent(location.pathname.replace(/^\/monitors\//, '')),
    currentStatus: null,
    loading: true,
    activeWindow: '24h',
    lastResponseMs: null,
    lastCheckedAt: null,
    detail: null,
    uptime24h: null,
    sampleCount: 0,
    lossCount: 0,
    chartMode: 'series',
    isLoadingDetail: false,
    hasData: false,
    timelineKind: '',    // '' | 'publicip' | 'border' — non-empty → state timeline
    tlLegend: [],        // [{ label, color, ms }, ...] for the timeline legend
    // Public-IP summary (recomputed per window from the timeline segments).
    ipCurrent: null,     // current address, or null when currently unreachable
    ipStableFor: '—',    // how long the current IP has held (in this window)
    ipChanges: 0,        // number of IP changes within the window
    ipDistinct: 0,       // distinct IPs seen in the window
    uptimeWindows: [
      { label: '1h',  key: 'uptime_1h',  value: null },
      { label: '24h', key: 'uptime_24h', value: null },
      { label: '7d',  key: 'uptime_7d',  value: null },
      { label: '30d', key: 'uptime_30d', value: null },
    ],

    async init() {
      document.title = `${this.monitorName} — Rusty Pingus`;
      const name = encodeURIComponent(this.monitorName);

      const [histRes, uptimeRes] = await Promise.all([
        fetch(`/api/monitors/${name}/history?limit=100`),
        fetch(`/api/monitors/${name}/uptime`),
      ]);
      if (!histRes.ok) { this.currentStatus = 'not_found'; this.loading = false; return; }

      const history = await histRes.json();
      const uptime = uptimeRes.ok ? await uptimeRes.json() : {};
      for (const w of this.uptimeWindows) w.value = uptime[w.key] ?? null;
      this.uptime24h = uptime.uptime_24h ?? null;

      const latest = history[0];
      this.currentStatus = latest ? latest.status : 'pending';
      this.lastResponseMs = latest ? latest.response_time_ms : null;
      this.lastCheckedAt = latest ? latest.checked_at : null;
      this.detail = latest ? latest.detail : null;
      const proto = latest ? latest.protocol : null;
      this.timelineKind = (proto === 'publicip' || proto === 'border') ? proto : '';

      this.loading = false;
      this.$nextTick(() => this.selectWindow(this.activeWindow));
    },

    // Select a window: it fixes the chart's time axis and loads the window's data.
    async selectWindow(label) {
      this.activeWindow = label;
      const secs = WINDOW_SECS[label] ?? 86400;
      windowTo = Date.now();
      windowFrom = windowTo - secs * 1000;
      await this.loadRange(windowFrom, windowTo, /* reset view to full window */ true);
    },

    // Fetch data at a resolution matched to the range, then render.
    async loadRange(fromMs, toMs, resetView = false) {
      const name = encodeURIComponent(this.monitorName);
      const to = Math.min(toMs, Date.now());
      const from = Math.min(fromMs, to - 1000);
      const fromIso = new Date(from).toISOString();
      const toIso = new Date(to).toISOString();

      const seq = ++refetchSeq;
      this.isLoadingDetail = true;
      try {
        // Public-IP and border monitors render a state timeline, not a response
        // line. That needs the per-probe `detail` (the IP / fault class), so we
        // always read raw history for them.
        if (this.timelineKind) {
          const histUrl = `/api/monitors/${name}/history?from=${encodeURIComponent(fromIso)}`
            + `&to=${encodeURIComponent(toIso)}&limit=${RAW_LIMIT}`;
          const rows = await (await fetch(histUrl)).json().catch(() => []);
          if (seq !== refetchSeq) return;
          const asc = [...rows].reverse();
          this.sampleCount = rows.length;
          tlSegments = buildSegments(asc, to, this.timelineKind, tlColorMap);
          this.tlLegend = buildLegend(tlSegments);
          this.lossCount = 0;
          this.hasData = tlSegments.length > 0;
          if (this.timelineKind === 'publicip') this._computeIpSummary(tlSegments);
          this.renderTimeline(resetView);
          return;
        }

        // Series first: it counts the whole range (uncapped), unlike /history.
        const serUrl = `/api/monitors/${name}/series?from=${encodeURIComponent(fromIso)}`
          + `&to=${encodeURIComponent(toIso)}&buckets=${BUCKET_TARGET}`;
        const series = await (await fetch(serUrl)).json().catch(() => []);
        if (seq !== refetchSeq) return;
        const total = series.reduce((acc, b) => acc + (b.count || 0), 0);

        let downFlags;
        if (total > 0 && total <= RAW_LIMIT) {
          const histUrl = `/api/monitors/${name}/history?from=${encodeURIComponent(fromIso)}`
            + `&to=${encodeURIComponent(toIso)}&limit=${RAW_LIMIT}`;
          const rows = await (await fetch(histUrl)).json().catch(() => []);
          if (seq !== refetchSeq) return;
          const asc = [...rows].reverse();
          lineData = asc.map(r => [Date.parse(r.checked_at), r.response_time_ms]);
          downFlags = asc.map(r => r.status === 'down');
          this.chartMode = 'raw';
          this.sampleCount = rows.length;
        } else {
          // For partial-loss buckets keep avg (line continues); fully-down buckets
          // have a null avg, so the line breaks there.
          lineData = series.map(b => [Date.parse(b.ts), b.avg_ms ?? null]);
          downFlags = series.map(b => b.up_ratio < 1);
          this.chartMode = 'series';
          this.sampleCount = total;
        }

        downIntervals = computeDownIntervals(lineData, downFlags);
        this.lossCount = downIntervals.length;
        this.hasData = lineData.length > 0;
        this.renderChart(resetView);
      } catch (e) {
        console.error('loadRange failed', e);
      } finally {
        if (seq === refetchSeq) this.isLoadingDetail = false;
      }
    },

    // Lazily create the ECharts instance and bind resize + zoom/pan refetch once.
    _ensureChart() {
      const el = document.getElementById('response-chart');
      if (!el) return null;
      if (!chart) {
        chart = echarts.init(el);
        if (!resizeBound) {
          window.addEventListener('resize', () => chart && chart.resize());
          resizeBound = true;
        }
        // Debounced refetch on any user zoom/pan/slider gesture.
        chart.on('datazoom', () => {
          if (isApplying) return;
          const dz = (chart.getOption().dataZoom || [])[0] || {};
          const start = dz.start ?? 0, end = dz.end ?? 100;
          const span = windowTo - windowFrom;
          const fromMs = windowFrom + span * (start / 100);
          const toMs = windowFrom + span * (end / 100);
          if (debounceTimer) clearTimeout(debounceTimer);
          debounceTimer = setTimeout(() => this.loadRange(fromMs, toMs, false), REFETCH_DEBOUNCE_MS);
        });
      }
      return chart;
    },

    _applyChartOption(option, resetView) {
      // notMerge:false so a zoom-triggered refetch keeps the current zoom window
      // (a fresh option has no explicit dataZoom extent and would snap to full).
      isApplying = true;
      chart.setOption(option, { notMerge: false });
      if (resetView) {
        chart.dispatchAction({ type: 'dataZoom', dataZoomIndex: 0, start: 0, end: 100 });
      }
      isApplying = false;
    },

    renderChart(resetView) {
      if (!this._ensureChart()) return;
      this._applyChartOption(this._chartOption(), resetView);
    },

    renderTimeline(resetView) {
      if (!this._ensureChart()) return;
      this._applyChartOption(this._timelineChartOption(), resetView);
    },

    // Derive the public-IP header metrics from the (window-scoped) segments.
    _computeIpSummary(segs) {
      const last = segs[segs.length - 1];
      this.ipCurrent = last ? last.state : null; // null → currently unreachable
      this.ipStableFor = last ? fmtDuration(Math.max(0, (last.end ?? Date.now()) - last.start)) : '—';
      let changes = 0, prev = null;
      const seen = new Set();
      for (const s of segs) {
        if (s.state == null) continue;
        seen.add(s.state);
        if (prev !== null && s.state !== prev) changes++;
        prev = s.state;
      }
      this.ipChanges = changes;
      this.ipDistinct = seen.size;
    },

    // A state timeline: each segment fills the span a value was in effect (an IP
    // for public-IP monitors; a fault class for border), colored per state. Built
    // as a single-row custom series over the same time axis/zoom.
    _timelineChartOption() {
      return {
        backgroundColor: 'transparent',
        textStyle: { color: '#94a3b8', fontFamily: 'Inter, system-ui, sans-serif' },
        grid: { left: 16, right: 16, top: 16, bottom: 64 },
        tooltip: {
          trigger: 'item',
          backgroundColor: '#1e293b',
          borderColor: '#334155',
          textStyle: { color: '#f1f5f9' },
          formatter: (p) => {
            const d = p.data;
            if (!d) return '';
            const start = new Date(d.value[0]).toLocaleString();
            const dur = fmtDuration(d.value[1] - d.value[0]);
            return `<b>${d.label}</b><br/>since ${start}<br/>for ${dur}`;
          },
        },
        xAxis: {
          type: 'time',
          min: windowFrom,
          max: windowTo,
          axisLine: { lineStyle: { color: '#334155' } },
          axisLabel: { color: '#64748b', fontSize: 11, hideOverlap: true },
          splitLine: { show: false },
        },
        yAxis: {
          type: 'category',
          data: ['IP'],
          show: false,
          axisLine: { show: false },
          axisTick: { show: false },
          axisLabel: { show: false },
        },
        dataZoom: [
          { type: 'inside', xAxisIndex: 0, filterMode: 'none', minValueSpan: 10_000 },
          { type: 'slider', xAxisIndex: 0, filterMode: 'none', height: 22, bottom: 16,
            borderColor: '#334155', fillerColor: 'rgba(34,211,238,0.15)',
            dataBackground: { lineStyle: { color: '#334155' }, areaStyle: { color: '#1e293b' } },
            textStyle: { color: '#64748b' }, handleStyle: { color: '#22d3ee' } },
        ],
        series: [{
          type: 'custom',
          clip: true,
          renderItem: (params, api) => {
            const start = api.coord([api.value(0), 0]);
            const end = api.coord([api.value(1), 0]);
            const bandH = api.size([0, 1])[1];
            const h = Math.max(10, Math.min(bandH * 0.55, 120));
            const x = start[0];
            const w = Math.max(1, end[0] - start[0]);
            const y = start[1] - h / 2;
            const seg = tlSegments[params.dataIndex] || {};
            const children = [{
              type: 'rect',
              shape: { x, y, width: w, height: h, r: 3 },
              style: api.style({ stroke: 'rgba(15,23,42,0.7)', lineWidth: 1 }),
            }];
            // Print the value on the band when it's wide enough to read.
            if (seg.label && w > 46) {
              children.push({
                type: 'text',
                style: {
                  text: seg.label,
                  x: x + w / 2,
                  y: start[1],
                  textAlign: 'center',
                  textVerticalAlign: 'middle',
                  fill: '#0f172a',
                  fontWeight: 600,
                  fontSize: 11,
                  fontFamily: 'Inter, system-ui, sans-serif',
                  width: w - 10,
                  overflow: 'truncate',
                  ellipsis: '…',
                },
              });
            }
            return { type: 'group', children };
          },
          encode: { x: [0, 1], y: 0 },
          data: tlSegments.map(s => ({
            value: [s.start, s.end, 0],
            label: s.label,
            itemStyle: { color: s.color },
          })),
        }],
      };
    },

    _chartOption() {
      // Sustained outages → fat labeled band; every outage → a thin vertical line
      // that stays visible (fixed pixel width) at any zoom and is hoverable.
      const sustained = downIntervals.filter(([s, e]) => e - s >= SUSTAINED_MS);
      const markArea = sustained.length ? {
        silent: false,
        itemStyle: { color: 'rgba(248,113,113,0.22)' },
        label: {
          show: true, color: '#fca5a5', fontSize: 10, position: 'insideTop',
          formatter: (p) => p.name || '',
        },
        data: sustained.map(([s, e]) => [{ xAxis: s, name: fmtOutage(s, e) }, { xAxis: e }]),
      } : undefined;
      const markLine = downIntervals.length ? {
        silent: false,
        symbol: 'none',
        lineStyle: { color: '#f87171', width: 1.5, opacity: 0.9 },
        label: { show: false },
        emphasis: { label: { show: true, color: '#fca5a5', formatter: (p) => p.name || '' } },
        data: downIntervals.map(([s, e]) => ({
          xAxis: s,
          name: (e - s >= SUSTAINED_MS) ? fmtOutage(s, e) : new Date(s).toLocaleString(),
        })),
      } : undefined;

      return {
        backgroundColor: 'transparent',
        textStyle: { color: '#94a3b8', fontFamily: 'Inter, system-ui, sans-serif' },
        grid: { left: 48, right: 16, top: 16, bottom: 64 },
        tooltip: {
          trigger: 'axis',
          backgroundColor: '#1e293b',
          borderColor: '#334155',
          textStyle: { color: '#f1f5f9' },
          formatter: (params) => {
            const arr = Array.isArray(params) ? params : [params];
            if (!arr.length) return '';
            const p = arr[0];
            const tsMs = p.axisValue != null ? p.axisValue
              : (Array.isArray(p.value) ? p.value[0] : null);
            const y = Array.isArray(p.value) ? p.value[1] : null;
            const time = tsMs != null ? new Date(tsMs).toLocaleString() : '';
            const body = y == null ? 'no response' : `${Math.round(y)} ms`;
            return time ? `${time}<br/>${body}` : body;
          },
        },
        xAxis: {
          type: 'time',
          min: windowFrom,
          max: windowTo,
          axisLine: { lineStyle: { color: '#334155' } },
          axisLabel: { color: '#64748b', fontSize: 11, hideOverlap: true },
          splitLine: { show: false },
        },
        yAxis: {
          type: 'value',
          min: 0,
          name: 'ms',
          nameTextStyle: { color: '#64748b' },
          axisLabel: { color: '#64748b', fontSize: 11 },
          splitLine: { lineStyle: { color: 'rgba(51,65,85,0.5)' } },
        },
        dataZoom: [
          { type: 'inside', xAxisIndex: 0, filterMode: 'none', minValueSpan: 10_000 },
          { type: 'slider', xAxisIndex: 0, filterMode: 'none', height: 22, bottom: 16,
            borderColor: '#334155', fillerColor: 'rgba(34,211,238,0.15)',
            dataBackground: { lineStyle: { color: '#334155' }, areaStyle: { color: '#1e293b' } },
            textStyle: { color: '#64748b' }, handleStyle: { color: '#22d3ee' } },
        ],
        series: [{
          type: 'line',
          showSymbol: false,
          connectNulls: false,
          smooth: true,
          lineStyle: { color: '#22d3ee', width: 2 },
          itemStyle: { color: '#22d3ee' },
          areaStyle: { color: 'rgba(34,211,238,0.10)' },
          data: lineData,
          markArea,
          markLine,
        }],
      };
    },

    resetZoom() {
      // Back to the full active window (also reloads it at window resolution).
      this.selectWindow(this.activeWindow);
    },

    // SVG gauge helpers
    gaugeOffset(value) {
      if (value == null) return CIRCUMFERENCE;
      return CIRCUMFERENCE * (1 - Math.max(0, Math.min(100, value)) / 100);
    },
    gaugeColor(value) {
      if (value == null) return '#334155';
      if (value >= 99) return '#34d399';
      if (value >= 90) return '#fbbf24';
      return '#f87171';
    },
    fmtLegendDuration(ms) {
      return fmtDuration(ms);
    },
    formatRelative(iso) {
      const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
      if (diff < 10) return 'just now';
      if (diff < 60) return `${diff}s ago`;
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return `${Math.floor(diff / 86400)}d ago`;
    },
  };
}

// ── State-timeline helpers (public-IP and border monitors) ───────────────────

// Distinct, legible-on-dark colors assigned to IPs in order of first appearance.
const IP_PALETTE = [
  '#22d3ee', '#a78bfa', '#f472b6', '#34d399', '#fbbf24',
  '#60a5fa', '#fb923c', '#4ade80', '#e879f9', '#2dd4bf',
];
const UNKNOWN_COLOR = '#475569'; // gray for down / no recorded state
// Border fault classes get semantic colors and human labels (not a palette).
const BORDER_COLORS = { ok: '#34d399', isp_down: '#fbbf24', lan_down: '#f87171' };
const BORDER_LABELS = { ok: 'OK', isp_down: 'ISP down', lan_down: 'LAN down' };

// The state a probe recorded: the leading token of its detail (the IP for
// public-IP; the fault class for border, which may carry a "—"/RTT suffix).
// Null for down/no-detail samples.
function extractState(row) {
  if (row.detail) {
    const tok = row.detail.trim().split(/\s+/)[0];
    if (tok) return tok;
  }
  return null;
}

function colorForState(kind, state, colorMap) {
  if (state == null) return UNKNOWN_COLOR;
  if (kind === 'border') return BORDER_COLORS[state] || UNKNOWN_COLOR;
  // public-IP: stable palette per distinct address.
  if (!colorMap.has(state)) colorMap.set(state, IP_PALETTE[colorMap.size % IP_PALETTE.length]);
  return colorMap.get(state);
}

function stateLabel(kind, state) {
  if (state == null) return kind === 'border' ? 'unknown' : 'no IP';
  if (kind === 'border') return BORDER_LABELS[state] || state;
  return state; // public-IP: the address as-is
}

// Collapse consecutive same-state samples (ascending) into [start, end) segments.
// Each runs from where the state first appears to the next change; the last
// extends to the window end (now).
function buildSegments(asc, toMs, kind, colorMap) {
  const segs = [];
  for (const r of asc) {
    const state = extractState(r);
    const t = Date.parse(r.checked_at);
    const last = segs[segs.length - 1];
    if (last && last.state === state) continue;
    if (last) last.end = t;
    segs.push({ state, start: t, end: null });
  }
  if (segs.length) segs[segs.length - 1].end = Math.min(Date.now(), toMs);
  for (const s of segs) {
    s.color = colorForState(kind, s.state, colorMap);
    s.label = stateLabel(kind, s.state);
  }
  return segs;
}

// One legend entry per distinct state, with its color and total time in effect.
function buildLegend(segs) {
  const m = new Map();
  for (const s of segs) {
    const cur = m.get(s.label) || { label: s.label, color: s.color, ms: 0 };
    cur.ms += Math.max(0, s.end - s.start);
    m.set(s.label, cur);
  }
  return [...m.values()];
}

function fmtDuration(ms) {
  const s = Math.max(1, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60), rs = s % 60;
  if (m < 60) return rs ? `${m}m${rs}s` : `${m}m`;
  const h = Math.floor(m / 60), rm = m % 60;
  if (h < 24) return rm ? `${h}h${rm}m` : `${h}h`;
  const d = Math.floor(h / 24), rh = h % 24;
  return rh ? `${d}d${rh}h` : `${d}d`;
}

// "14:03:20 · 1m40s" — outage start time + duration, for sustained-outage labels.
function fmtOutage(startMs, endMs) {
  return `${new Date(startMs).toLocaleTimeString()} · ${fmtDuration(endMs - startMs)}`;
}

// Merge consecutive down samples into [startMs, endMs] spans for the red bands.
function computeDownIntervals(lineData, downFlags) {
  const out = [];
  const n = lineData.length;
  for (let i = 0; i < n; i++) {
    if (!downFlags[i]) continue;
    const start = lineData[i][0];
    // End at the next sample's time (so the band has width), or extrapolate.
    let end;
    if (i + 1 < n) end = lineData[i + 1][0];
    else if (i > 0) end = start + (lineData[i][0] - lineData[i - 1][0]);
    else end = start + 1000;
    const last = out[out.length - 1];
    if (last && start <= last[1]) last[1] = Math.max(last[1], end);
    else out.push([start, end]);
  }
  return out;
}
