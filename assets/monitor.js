function monitorDetail() {
  const CIRCUMFERENCE = 2 * Math.PI * 40; // r=40 → ≈251.2
  const WINDOW_SECS = { '1h': 3600, '24h': 86400, '7d': 604800, '30d': 2592000 };
  const RAW_LIMIT = 1000;   // max raw probes to render individually
  const BUCKET_TARGET = 300;
  const REFETCH_DEBOUNCE_MS = 300;
  const LIVE_REFRESH_DEBOUNCE_MS = 1000; // coalesce a burst of fast probes into one reload
  const POLL_MS = 30_000;                // fallback cadence while SSE is unavailable
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
  // Anchor for all data-range math: the newest server-recorded probe time (ms).
  // Probe rows are stamped on the SERVER clock; the browser clock can differ
  // (WSL2 host/guest drift was observed ~4h40m). Building windows from the
  // server's data time keeps ranged queries inside the data regardless of skew.
  let serverNowMs = null;   // null until first data → _anchorMs() falls back to Date.now()
  let lastTimelineValue = null; // last rendered state-timeline value (IP / fault class)
  let refetchSeq = 0;
  let debounceTimer = null;
  let isApplying = false;   // guard: programmatic setOption/dispatch must not refetch
  let resizeBound = false;
  // Live updates: kept OUT of Alpine's reactive object (like the dashboard) — a
  // proxied EventSource is a known footgun.
  let eventSource = null;
  let liveRefreshTimer = null;
  let pollTimer = null;
  // Traceroute view: the brush ECharts instance is kept OUT of Alpine (circular
  // refs), like the main chart. `traceTimer` debounces brush-driven reloads.
  let traceBrush = null;
  let traceTimer = null;
  let traceApplying = false; // guard: programmatic brush setOption must not refetch
  let hopChart = null;       // the per-hop latency graph (min–max band + avg line)

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
    // Traceroute view (protocol === 'traceroute').
    isTraceroute: false,
    traceHops: [],       // [{ hop_no, addr, reachable, min_ms, avg_ms, max_ms, loss, samples }]
    traceLoading: false,
    traceFrom: null, traceTo: null,              // ms; the active (brushed) range
    traceExtentFrom: null, traceExtentTo: null,  // ms; retained-data bounds
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

      const latest = history[0]; // history is newest-first
      this.currentStatus = latest ? latest.status : 'pending';
      this.lastResponseMs = latest ? latest.response_time_ms : null;
      this.lastCheckedAt = latest ? latest.checked_at : null;
      this.detail = latest ? latest.detail : null;
      // Anchor windows to the newest server-recorded probe time (falls back to
      // the browser clock only while there is no data).
      if (latest) this._bumpServerNow(latest.checked_at);
      const proto = latest ? latest.protocol : null;
      this.timelineKind = (proto === 'publicip' || proto === 'border') ? proto : '';
      this.isTraceroute = proto === 'traceroute';

      this.loading = false;
      if (this.isTraceroute) {
        this.$nextTick(() => this._initTraceroute());
      } else {
        this.$nextTick(() => this.selectWindow(this.activeWindow));
      }

      // Stay current as probes arrive (matches the dashboard): subscribe to the
      // live stream, fall back to polling, and tear down on navigation.
      this.connectLive();
      window.addEventListener('beforeunload', () => this._teardownLive());
    },

    // ── Live updates ──────────────────────────────────────────────────────────
    // Subscribe to the shared SSE feed and refresh in place for THIS monitor.
    // Header fields update instantly from the event; the visible range is
    // reloaded on a debounce so a just-started monitor's data fills in without a
    // manual reload. A poll covers any window where the stream is unavailable.
    connectLive() {
      eventSource = window.RP.subscribeStatus({
        onOpen: () => this._stopPoll(),            // stream live → poll not needed
        onError: () => this._startPoll(),          // browser reconnects; poll covers the gap
        onUpdate: (u) => {
          if (u.name !== this.monitorName) return; // ignore other monitors
          this._onLiveUpdate(u);
        },
      });
      if (!eventSource) this._startPoll();          // no SSE support → poll-only
    },

    // Update the header/strip fields directly from a status payload.
    _applyLiveStatus(u) {
      this.currentStatus = u.status;
      this.lastResponseMs = u.response_time_ms;
      this.lastCheckedAt = u.last_checked_at;
      this.detail = u.detail;
      this._bumpServerNow(u.last_checked_at); // advance the anchor with fresh data
    },

    // The server-data anchor: newest known probe time, or the browser clock when
    // no probe data has been seen yet.
    _anchorMs() {
      return serverNowMs ?? Date.now();
    },
    // Advance the anchor to the newest server timestamp seen (never rewind).
    _bumpServerNow(iso) {
      if (!iso) return;
      const ms = Date.parse(iso);
      if (!Number.isNaN(ms)) serverNowMs = Math.max(serverNowMs ?? 0, ms);
    },

    // Apply a live status payload and refresh the view — but for state-timeline
    // monitors only reload the chart when the tracked value actually changes
    // (suppresses per-probe flashing). Response-time monitors always refresh.
    _onLiveUpdate(u) {
      this._applyLiveStatus(u);
      if (this.isTraceroute) {
        // Header updates live; the hop table re-aggregates only when the brush is
        // parked at the live edge (viewing "now"), so a parked historical view is
        // left undisturbed.
        this._scheduleTraceLive();
        return;
      }
      if (this.timelineKind) {
        const incoming = extractState({ detail: u.detail ?? '' });
        if (incoming !== lastTimelineValue) this._scheduleLiveRefresh();
      } else {
        this._scheduleLiveRefresh();
      }
    },

    // Reload the visible data, debounced, without stealing the user's view.
    _scheduleLiveRefresh() {
      if (liveRefreshTimer) clearTimeout(liveRefreshTimer);
      liveRefreshTimer = setTimeout(() => {
        if (isApplying) { this._scheduleLiveRefresh(); return; } // retry after apply
        if (debounceTimer) return; // a user zoom/pan refetch is queued; it'll refresh
        this._refreshVisible();
      }, LIVE_REFRESH_DEBOUNCE_MS);
    },

    // Refresh whatever is on screen. At the full active window (not zoomed) slide
    // it to include "now" so fresh probes appear; if the user has zoomed/panned,
    // keep their exact range (and position).
    _refreshVisible() {
      if (windowFrom == null) return;                 // window not initialized yet
      if (!chart) { this.selectWindow(this.activeWindow); return; }
      const dz = (chart.getOption().dataZoom || [])[0] || {};
      const start = dz.start ?? 0, end = dz.end ?? 100;
      const atFull = start <= 0.05 && end >= 99.95;
      if (atFull) {
        this.selectWindow(this.activeWindow);         // slides to now; view stays full
      } else {
        const span = windowTo - windowFrom;
        const fromMs = windowFrom + span * (start / 100);
        const toMs = windowFrom + span * (end / 100);
        this.loadRange(fromMs, toMs, false);          // preserve zoom/pan position
      }
    },

    _startPoll() {
      if (pollTimer) return;                          // already polling
      pollTimer = setInterval(() => this._pollOnce(), POLL_MS);
    },
    _stopPoll() {
      if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
    },
    async _pollOnce() {
      try {
        const name = encodeURIComponent(this.monitorName);
        const rows = await (await fetch(`/api/monitors/${name}/history?limit=1`)).json();
        const latest = Array.isArray(rows) ? rows[0] : null;
        if (latest) this._onLiveUpdate({
          name: this.monitorName,
          status: latest.status,
          response_time_ms: latest.response_time_ms,
          last_checked_at: latest.checked_at,
          detail: latest.detail,
        });
      } catch (e) { /* poll is best-effort */ }
    },
    _teardownLive() {
      if (eventSource) { eventSource.close(); eventSource = null; }
      this._stopPoll();
      if (liveRefreshTimer) { clearTimeout(liveRefreshTimer); liveRefreshTimer = null; }
      if (traceTimer) { clearTimeout(traceTimer); traceTimer = null; }
    },

    // Select a window: it fixes the chart's time axis and loads the window's data.
    async selectWindow(label) {
      this.activeWindow = label;
      const secs = WINDOW_SECS[label] ?? 86400;
      windowTo = this._anchorMs(); // server data time, not the browser clock
      windowFrom = windowTo - secs * 1000;
      await this.loadRange(windowFrom, windowTo, /* reset view to full window */ true);
    },

    // Fetch data at a resolution matched to the range, then render.
    async loadRange(fromMs, toMs, resetView = false) {
      const name = encodeURIComponent(this.monitorName);
      const to = Math.min(toMs, this._anchorMs()); // clamp to server data time, not browser clock
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
          if (rows[0]) this._bumpServerNow(rows[0].checked_at); // rows are newest-first
          const asc = [...rows].reverse();
          this.sampleCount = rows.length;
          tlSegments = buildSegments(asc, to, this.timelineKind, tlColorMap);
          // Remember the value currently on screen so live events can detect a
          // real change and only then reload the timeline (no per-probe flashing).
          lastTimelineValue = tlSegments.length ? tlSegments[tlSegments.length - 1].state : null;
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
        const lastBucket = series[series.length - 1]; // series is ascending by ts
        if (lastBucket) this._bumpServerNow(lastBucket.ts);
        const total = series.reduce((acc, b) => acc + (b.count || 0), 0);

        let downFlags;
        if (total > 0 && total <= RAW_LIMIT) {
          const histUrl = `/api/monitors/${name}/history?from=${encodeURIComponent(fromIso)}`
            + `&to=${encodeURIComponent(toIso)}&limit=${RAW_LIMIT}`;
          const rows = await (await fetch(histUrl)).json().catch(() => []);
          if (seq !== refetchSeq) return;
          if (rows[0]) this._bumpServerNow(rows[0].checked_at); // rows are newest-first
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
      this.ipStableFor = last ? fmtDuration(Math.max(0, (last.end ?? this._anchorMs()) - last.start)) : '—';
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

    // ── Traceroute view ───────────────────────────────────────────────────────
    // A traceroute monitor shows a per-hop table (latency waterfall) over a
    // brush-selected time range within its retained data, not the line/timeline
    // chart or the fixed uptime windows.
    async _initTraceroute() {
      await this._loadTraceExtent();
      const now = this._anchorMs();
      this.traceExtentTo = this.traceExtentTo ?? now;
      this.traceExtentFrom = this.traceExtentFrom ?? (this.traceExtentTo - 3_600_000);
      this.traceFrom = this.traceExtentFrom;
      this.traceTo = this.traceExtentTo;
      await this._loadTraceHops();
      this.$nextTick(() => this._renderBrush());
    },

    async _loadTraceExtent() {
      const name = encodeURIComponent(this.monitorName);
      try {
        const ext = await (await fetch(`/api/monitors/${name}/traceroute/extent`)).json();
        this.traceExtentFrom = ext.from ? Date.parse(ext.from) : null;
        this.traceExtentTo = ext.to ? Date.parse(ext.to) : null;
      } catch (e) { /* no data yet; bounds stay null */ }
    },

    // Fetch per-hop aggregates for the active [traceFrom, traceTo] range and
    // compute the bar-scaling maximum (the slowest hop avg in view).
    async _loadTraceHops() {
      if (this.traceFrom == null || this.traceTo == null) return;
      const name = encodeURIComponent(this.monitorName);
      const fromIso = new Date(this.traceFrom).toISOString();
      const toIso = new Date(this.traceTo).toISOString();
      this.traceLoading = true;
      try {
        const hops = await (await fetch(
          `/api/monitors/${name}/traceroute?from=${encodeURIComponent(fromIso)}&to=${encodeURIComponent(toIso)}`
        )).json();
        this.traceHops = Array.isArray(hops) ? hops : [];
        this.hasData = this.traceHops.length > 0;
        // Re-render the latency graph once the table rows (which set its height
        // via the spanning cell) have been laid out.
        this.$nextTick(() => this._renderHopChart());
      } catch (e) {
        this.traceHops = []; this.hasData = false;
      } finally {
        this.traceLoading = false;
      }
    },

    // The resizable brush: an ECharts time slider spanning the retained extent.
    // Moving/resizing it sets the active range and re-aggregates the table.
    _renderBrush() {
      const el = document.getElementById('trace-brush');
      if (!el || !window.echarts) return;
      const self = this;
      const lo = this.traceExtentFrom ?? (this._anchorMs() - 3_600_000);
      const hi = this.traceExtentTo ?? this._anchorMs();
      const clamp = (x) => Math.max(0, Math.min(100, x));
      const span = Math.max(1, hi - lo);

      if (!traceBrush) {
        traceBrush = echarts.init(el);
        window.addEventListener('resize', () => traceBrush && traceBrush.resize());
        traceBrush.on('datazoom', () => {
          if (traceApplying) return; // programmatic update, not a user gesture
          const dz = (traceBrush.getOption().dataZoom || [])[0] || {};
          const lo2 = self.traceExtentFrom ?? lo;
          const hi2 = self.traceExtentTo ?? hi;
          const span2 = Math.max(1, hi2 - lo2);
          self.traceFrom = lo2 + span2 * ((dz.start ?? 0) / 100);
          self.traceTo = lo2 + span2 * ((dz.end ?? 100) / 100);
          if (traceTimer) clearTimeout(traceTimer);
          traceTimer = setTimeout(() => self._loadTraceHops(), 300);
        });
      }

      const startPct = clamp(((this.traceFrom - lo) / span) * 100);
      const endPct = clamp(((this.traceTo - lo) / span) * 100);
      traceApplying = true;
      traceBrush.setOption({
        backgroundColor: 'transparent',
        grid: { left: 8, right: 8, top: 6, bottom: 26 },
        xAxis: {
          type: 'time', min: lo, max: hi,
          axisLine: { lineStyle: { color: '#334155' } },
          axisLabel: { color: '#64748b', fontSize: 10, hideOverlap: true },
          splitLine: { show: false },
        },
        yAxis: { type: 'value', show: false, min: 0, max: 1 },
        dataZoom: [
          { type: 'inside', xAxisIndex: 0, filterMode: 'none' },
          { type: 'slider', xAxisIndex: 0, filterMode: 'none', height: 18, bottom: 2,
            start: startPct, end: endPct,
            borderColor: '#334155', fillerColor: 'rgba(34,211,238,0.15)',
            dataBackground: { lineStyle: { color: '#334155' }, areaStyle: { color: '#1e293b' } },
            textStyle: { color: '#64748b' }, handleStyle: { color: '#22d3ee' } },
        ],
        series: [{
          type: 'line', showSymbol: false, lineStyle: { width: 0 },
          areaStyle: { color: 'rgba(34,211,238,0.08)' }, data: [[lo, 1], [hi, 1]],
        }],
      }, { notMerge: false });
      traceApplying = false;
    },

    // On a live probe: advance the extent, and if the brush is parked at the live
    // edge, slide it to "now" and refresh — otherwise leave a historical view be.
    _scheduleTraceLive() {
      const now = this._anchorMs();
      const live = this.traceTo == null || this.traceExtentTo == null
        || this.traceTo >= this.traceExtentTo - 1000;
      this.traceExtentTo = Math.max(this.traceExtentTo ?? now, now);
      if (this.traceExtentFrom == null) this.traceExtentFrom = now;
      if (!live) return;
      this.traceTo = this.traceExtentTo;
      if (traceTimer) clearTimeout(traceTimer);
      traceTimer = setTimeout(() => { this._loadTraceHops(); this._renderBrush(); }, 1000);
    },

    // The latency graph: one chart aligned to the hop rows. Each hop is a
    // category (top→bottom), the value axis is latency; per hop a horizontal
    // min–max band is drawn and the averages are connected by a line with dots.
    // The chart fills a table cell that spans all body rows, so its category
    // bands line up with the rows.
    _renderHopChart() {
      const el = document.getElementById('hop-chart');
      if (!el || !window.echarts) return;
      const hops = this.traceHops;
      if (!hops.length) return;
      // A %-height div inside a <td> collapses to content height, so size the
      // chart element explicitly to the spanning cell (whose height the sibling
      // rows fix). This is also what aligns the category bands to the rows.
      const td = el.parentElement;
      if (td && td.clientHeight) el.style.height = td.clientHeight + 'px';
      // If Alpine recreated the cell element, rebind to the live node.
      if (hopChart && hopChart.getDom() !== el) { hopChart.dispose(); hopChart = null; }
      if (!hopChart) {
        hopChart = echarts.init(el);
        window.addEventListener('resize', () => { if (hopChart) hopChart.resize(); });
      } else {
        hopChart.resize(); // the spanning cell's height changes with hop count
      }

      let maxX = 0;
      for (const h of hops) { const v = h.max_ms ?? h.avg_ms ?? 0; if (v > maxX) maxX = v; }
      maxX = maxX > 0 ? maxX * 1.1 : 1;

      const cats = hops.map(h => String(h.hop_no));
      const bandData = hops.map((h, i) => [h.min_ms, h.max_ms, i]);
      const avgData = hops.map(h => h.avg_ms ?? null);
      const fmt = (v) => v == null ? '—' : (v < 10 ? v.toFixed(2) : Math.round(v).toLocaleString()) + ' ms';

      hopChart.setOption({
        backgroundColor: 'transparent',
        grid: { top: 0, bottom: 0, left: 8, right: 14 },
        tooltip: {
          trigger: 'axis', axisPointer: { type: 'shadow' },
          backgroundColor: '#1e293b', borderColor: '#334155', textStyle: { color: '#f1f5f9' },
          formatter: (ps) => {
            const i = ps[0].dataIndex; const h = hops[i];
            return `<b>hop ${h.hop_no}</b> ${h.addr || '*'}<br/>`
              + `avg ${fmt(h.avg_ms)}<br/>min ${fmt(h.min_ms)} · max ${fmt(h.max_ms)}`;
          },
        },
        xAxis: { type: 'value', min: 0, max: maxX, show: false },
        yAxis: {
          type: 'category', inverse: true, data: cats, boundaryGap: true,
          show: false, axisLine: { show: false }, axisTick: { show: false },
        },
        series: [
          {
            type: 'custom', z: 1, encode: { x: [0, 1], y: 2 }, data: bandData,
            renderItem: (params, api) => {
              const min = api.value(0), max = api.value(1), idx = api.value(2);
              if (min == null || max == null || isNaN(min) || isNaN(max)) return;
              const p1 = api.coord([min, idx]);
              const p2 = api.coord([max, idx]);
              const h = 7;
              return {
                type: 'rect',
                shape: { x: p1[0], y: p1[1] - h / 2, width: Math.max(2, p2[0] - p1[0]), height: h, r: 3 },
                style: { fill: 'rgba(34,211,238,0.22)' },
              };
            },
          },
          {
            type: 'line', z: 2, data: avgData, connectNulls: false,
            symbol: 'circle', symbolSize: 8, showSymbol: true,
            lineStyle: { color: '#22d3ee', width: 2 }, itemStyle: { color: '#22d3ee' },
          },
        ],
      }, { notMerge: true });
      hopChart.resize();
    },
    traceFmtMs(v) {
      if (v == null) return '—';
      return (v < 10 ? v.toFixed(1) : Math.round(v).toLocaleString()) + ' ms';
    },
    traceRangeLabel() {
      if (this.traceFrom == null || this.traceTo == null) return '';
      const f = new Date(this.traceFrom).toLocaleString();
      const t = new Date(this.traceTo).toLocaleString();
      return `${f} → ${t}`;
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
      return window.RP.formatRelative(iso);
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
  // toMs is the server-anchored window end; don't clip with the browser clock.
  if (segs.length) segs[segs.length - 1].end = toMs;
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
