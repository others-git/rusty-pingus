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
    uptime24h: null,
    sampleCount: 0,
    lossCount: 0,
    chartMode: 'series',
    isLoadingDetail: false,
    hasData: false,
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

    renderChart(resetView) {
      const el = document.getElementById('response-chart');
      if (!el) return;
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

      isApplying = true;
      chart.setOption(this._chartOption(), { notMerge: false });
      if (resetView) {
        chart.dispatchAction({ type: 'dataZoom', dataZoomIndex: 0, start: 0, end: 100 });
      }
      isApplying = false;
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

function fmtDuration(ms) {
  const s = Math.max(1, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60), rs = s % 60;
  if (m < 60) return rs ? `${m}m${rs}s` : `${m}m`;
  const h = Math.floor(m / 60), rm = m % 60;
  return rm ? `${h}h${rm}m` : `${h}h`;
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
