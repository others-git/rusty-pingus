function monitorDetail() {
  const CIRCUMFERENCE = 2 * Math.PI * 40; // r=40 → ≈251.2
  const WINDOW_SECS = { '1h': 3600, '24h': 86400, '7d': 604800, '30d': 2592000 };
  const RAW_LIMIT = 1000;   // max raw probes to render individually
  const BUCKET_TARGET = 300;
  const REFETCH_DEBOUNCE_MS = 300;

  return {
    monitorName: decodeURIComponent(location.pathname.replace(/^\/monitors\//, '')),
    currentStatus: null,
    loading: true,
    history: [],
    points: [],
    lossPoints: [],
    lossCount: 0,
    chart: null,
    chartMode: 'series',     // 'series' | 'raw'
    activeWindow: '24h',
    lastResponseMs: null,
    lastCheckedAt: null,
    uptime24h: null,
    sampleCount: 0,
    isLoadingDetail: false,
    windowFrom: null,        // ms; full extent of the active window (max zoom-out)
    windowTo: null,
    _isApplying: false,      // guard: programmatic chart updates must not refetch
    _refetchSeq: 0,          // stale-response guard
    _debounceTimer: null,
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

      if (!histRes.ok) {
        this.currentStatus = 'not_found';
        this.loading = false;
        return;
      }

      this.history = await histRes.json();
      const uptime = uptimeRes.ok ? await uptimeRes.json() : {};
      for (const w of this.uptimeWindows) w.value = uptime[w.key] ?? null;
      this.uptime24h = uptime.uptime_24h ?? null;

      const latest = this.history[0];
      this.currentStatus = latest ? latest.status : 'pending';
      this.lastResponseMs = latest ? latest.response_time_ms : null;
      this.lastCheckedAt = latest ? latest.checked_at : null;

      this.loading = false;
      this.$nextTick(() => this.selectWindow(this.activeWindow));
    },

    // Select a window: fetch its data and fit the chart's x-axis to that data so
    // it fills the plot area. The window only governs what data is fetched — it
    // does not pin the axis or the pan range.
    async selectWindow(label) {
      this.activeWindow = label;
      const secs = WINDOW_SECS[label] ?? 86400;
      this.windowTo = Date.now();
      this.windowFrom = this.windowTo - secs * 1000;
      await this.loadRange(this.windowFrom, this.windowTo, /* fit axis to data */ true);
    },

    // Fetch data at a resolution matched to the visible range and update the chart.
    // Uses raw probes when the range holds few enough; otherwise aggregated buckets.
    async loadRange(fromMs, toMs, fit = false) {
      const name = encodeURIComponent(this.monitorName);
      const to = Math.min(toMs, Date.now());
      const from = Math.min(fromMs, to - 1000);
      const fromIso = new Date(from).toISOString();
      const toIso = new Date(to).toISOString();

      const seq = ++this._refetchSeq;
      this.isLoadingDetail = true;
      try {
        // Fetch the aggregated series first. It scans the whole range (uncapped),
        // so the summed bucket counts give the true number of probes in range —
        // unlike /history, whose limit is server-capped at 1000.
        const serUrl = `/api/monitors/${name}/series?from=${encodeURIComponent(fromIso)}`
          + `&to=${encodeURIComponent(toIso)}&buckets=${BUCKET_TARGET}`;
        const serRes = await fetch(serUrl);
        const series = serRes.ok ? await serRes.json() : [];
        if (seq !== this._refetchSeq) return; // a newer request superseded us
        const total = series.reduce((acc, b) => acc + (b.count || 0), 0);

        if (total > 0 && total <= RAW_LIMIT) {
          // Sparse enough → fetch every probe in range and show them individually.
          const histUrl = `/api/monitors/${name}/history?from=${encodeURIComponent(fromIso)}`
            + `&to=${encodeURIComponent(toIso)}&limit=${RAW_LIMIT}`;
          const histRes = await fetch(histUrl);
          const rows = histRes.ok ? await histRes.json() : [];
          if (seq !== this._refetchSeq) return;
          const asc = [...rows].reverse(); // history is newest-first
          this.points = asc.map(r => ({ x: r.checked_at, y: r.response_time_ms }));
          this._pointMeta = asc.map(r => ({ down: r.status === 'down', reason: r.failure_reason }));
          this.chartMode = 'raw';
          this.sampleCount = rows.length;
        } else {
          this.points = series.map(b => ({ x: b.ts, y: b.avg_ms }));
          this._pointMeta = series.map(b => ({
            down: b.up_ratio < 1, count: b.count, min: b.min_ms, max: b.max_ms, up_ratio: b.up_ratio,
          }));
          this.chartMode = 'series';
          this.sampleCount = total;
        }
        this.renderChart(fit);
      } catch (e) {
        console.error('loadRange failed', e);
      } finally {
        if (seq === this._refetchSeq) this.isLoadingDetail = false;
      }
    },

    scheduleRefetch(fromMs, toMs) {
      if (this._debounceTimer) clearTimeout(this._debounceTimer);
      this._debounceTimer = setTimeout(() => this.loadRange(fromMs, toMs, false), REFETCH_DEBOUNCE_MS);
    },

    resetZoom() {
      // Snap back to the full active window (also reloads it at window resolution).
      this.selectWindow(this.activeWindow);
    },

    // Down probes (raw) / buckets with loss (series) plotted as red markers along
    // the baseline so packet loss is visible on the chart.
    _computeLoss() {
      const pts = [];
      for (let i = 0; i < this.points.length; i++) {
        const m = this._pointMeta[i];
        if (m && m.down) pts.push({ x: this.points[i].x, y: 0 });
      }
      this.lossPoints = pts;
      this.lossCount = pts.length;
    },

    _mainRadius() {
      return this.points.length > 120 ? 0 : 2;
    },

    _lossRadius() {
      return this.chartMode === 'raw' ? 4 : 3;
    },

    // [minMs, maxMs] of the loaded points, with small padding so edge points
    // aren't clipped. Used to fit the axis to the data on a fresh window load.
    _dataExtent() {
      if (!this.points.length) return null;
      const first = Date.parse(this.points[0].x);
      const last = Date.parse(this.points[this.points.length - 1].x);
      const pad = Math.max(1000, (last - first) * 0.02);
      return [first - pad, last + pad];
    },

    renderChart(fit = false) {
      const canvas = document.getElementById('response-chart');
      if (!canvas) return;

      this._computeLoss();

      // Always update the single chart instance in place — never destroy/recreate.
      // Destroying nulls the canvas context while the zoom plugin may still have a
      // throttled update pending, which then crashes in clipArea (ctx null).
      if (this.chart) {
        this._isApplying = true;
        this.chart.data.datasets[0].data = this.points;
        this.chart.data.datasets[0].pointRadius = this._mainRadius();
        this.chart.data.datasets[1].data = this.lossPoints;
        this.chart.data.datasets[1].pointRadius = this._lossRadius();
        // Only a fresh window load re-fits the axis to its data; zoom/pan refetches
        // leave the view exactly where the user's gesture left it.
        if (fit) {
          const ext = this._dataExtent();
          this.chart.options.scales.x.min = ext ? ext[0] : undefined;
          this.chart.options.scales.x.max = ext ? ext[1] : undefined;
        }
        this.chart.update('none');
        this._isApplying = false;
        return;
      }

      if (!this.points.length) return;

      const ext = this._dataExtent(); // fit the initial view to the loaded data

      // NOTE: no `fill` — Chart.js's DatasetController.initialize() calls
      // isPluginEnabled('filler') when fill is set, and the zoom plugin's
      // re-entrant update() can reconstruct the controller while the plugin
      // cache is momentarily undefined, throwing. A plain line avoids that path.
      this.chart = new Chart(canvas, {
        type: 'line',
        data: {
          datasets: [
            {
              label: 'Response (ms)',
              data: this.points,
              borderColor: '#22d3ee',
              backgroundColor: 'rgba(34,211,238,0.15)',
              borderWidth: 2,
              pointBackgroundColor: '#22d3ee',
              pointRadius: this._mainRadius(),
              pointHoverRadius: 4,
              tension: 0.3,
              fill: false,
              spanGaps: true,
            },
            {
              label: 'Packet loss',
              data: this.lossPoints,
              showLine: false,
              pointStyle: 'circle',
              pointBackgroundColor: '#f87171',
              pointBorderColor: '#f87171',
              pointRadius: this._lossRadius(),
              pointHoverRadius: 5,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          animation: false,
          plugins: {
            legend: { display: false },
            tooltip: {
              mode: 'index',
              intersect: false,
              backgroundColor: '#1e293b',
              titleColor: '#94a3b8',
              bodyColor: '#f1f5f9',
              borderColor: '#334155',
              borderWidth: 1,
              callbacks: {
                title: items => items.length ? new Date(items[0].parsed.x).toLocaleString() : '',
                label: c => {
                  if (c.datasetIndex === 1) {
                    return this.chartMode === 'raw' ? 'packet lost (no reply)' : 'loss in this interval';
                  }
                  return this._tooltipLabel(c.dataIndex, c.parsed.y);
                },
              },
            },
            zoom: {
              // NOTE: pinch is intentionally omitted — it requires Hammer.js, and
              // enabling it without Hammer loaded throws during the plugin's event
              // setup, which prevents the mouse drag-to-pan listeners from attaching.
              zoom: {
                wheel: { enabled: true },
                mode: 'x',
                onZoomComplete: ({ chart }) => this._onViewChange(chart),
              },
              pan: {
                enabled: true,
                mode: 'x',
                onPanComplete: ({ chart }) => this._onViewChange(chart),
              },
              // No `limits`: clamping the view to the window leaves no room to pan.
              // minRange caps how far the user can zoom in.
              limits: {
                x: { minRange: 10_000 },
              },
            },
          },
          scales: {
            x: {
              type: 'time',
              min: ext ? ext[0] : undefined,
              max: ext ? ext[1] : undefined,
              ticks: { maxTicksLimit: 8, color: '#64748b', font: { size: 11 } },
              grid: { display: false },
              border: { display: false },
            },
            y: {
              beginAtZero: true,
              ticks: { color: '#64748b', font: { size: 11 } },
              grid: { color: 'rgba(51,65,85,0.5)' },
              border: { display: false },
            },
          },
        },
      });
    },

    // Refetch for the newly visible range — ignore programmatic updates.
    _onViewChange(chart) {
      if (this._isApplying) return;
      const { min, max } = chart.scales.x;
      if (min == null || max == null) return;
      this.scheduleRefetch(min, max);
    },

    _tooltipLabel(i, y) {
      const m = this._pointMeta[i] || {};
      if (this.chartMode === 'raw') {
        const out = [y != null ? `${y} ms` : (m.down ? 'down' : '—')];
        if (m.down && m.reason) out.push(m.reason);
        return out;
      }
      const out = [`avg ${y != null ? Math.round(y) : '—'} ms`];
      if (m.min != null && m.max != null) out.push(`min ${m.min} / max ${m.max} ms`);
      if (m.count != null) out.push(`${m.count} sample${m.count === 1 ? '' : 's'}`);
      if (m.up_ratio != null && m.up_ratio < 1) out.push(`${(m.up_ratio * 100).toFixed(0)}% up`);
      return out;
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

    formatTime(iso) {
      return new Date(iso).toLocaleString();
    },

    formatRelative(iso) {
      const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
      if (diff < 10) return 'just now';
      if (diff < 60) return `${diff}s ago`;
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return `${Math.floor(diff / 86400)}d ago`;
    },

    _pointMeta: [],
  };
}
