function monitorDetail() {
  const CIRCUMFERENCE = 2 * Math.PI * 40; // r=40 → ≈251.2

  return {
    monitorName: decodeURIComponent(location.pathname.replace(/^\/monitors\//, '')),
    currentStatus: null,
    loading: true,
    history: [],
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

      // Populate gauge windows
      for (const w of this.uptimeWindows) {
        w.value = uptime[w.key] ?? null;
      }

      this.currentStatus = this.history.length ? this.history[0].status : 'pending';
      this.loading = false;

      // Chart must render after DOM update (Alpine flushes after this tick)
      this.$nextTick(() => this.renderChart());
    },

    // SVG gauge helpers
    gaugeOffset(value) {
      if (value == null) return CIRCUMFERENCE;
      return CIRCUMFERENCE * (1 - Math.max(0, Math.min(100, value)) / 100);
    },

    gaugeColor(value) {
      if (value == null) return '#334155'; // slate-700
      if (value >= 99) return '#34d399';   // emerald-400
      if (value >= 90) return '#fbbf24';   // amber-400
      return '#f87171';                     // red-400
    },

    formatTime(iso) {
      return new Date(iso).toLocaleString();
    },

    renderChart() {
      const canvas = document.getElementById('response-chart');
      if (!canvas || !this.history.length) return;

      const ctx = canvas.getContext('2d');

      // Gradient fill
      const gradient = ctx.createLinearGradient(0, 0, 0, 240);
      gradient.addColorStop(0, 'rgba(34,211,238,0.25)');
      gradient.addColorStop(1, 'rgba(34,211,238,0.01)');

      // Data is newest-first; reverse for chronological chart
      const sorted = [...this.history].reverse();
      const labels = sorted.map(r => new Date(r.checked_at).toLocaleTimeString());
      const data   = sorted.map(r => r.response_time_ms);

      new Chart(canvas, {
        type: 'line',
        data: {
          labels,
          datasets: [{
            label: 'Response Time (ms)',
            data,
            borderColor: '#22d3ee',
            backgroundColor: gradient,
            borderWidth: 2,
            pointRadius: data.length > 60 ? 0 : 2,
            pointHoverRadius: 4,
            tension: 0.3,
            fill: true,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
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
                label: ctx => `${ctx.parsed.y != null ? ctx.parsed.y + 'ms' : '—'}`,
              },
            },
          },
          scales: {
            x: {
              ticks: { maxTicksLimit: 8, color: '#64748b', font: { size: 11 } },
              grid: { display: false },
              border: { display: false },
            },
            y: {
              ticks: { color: '#64748b', font: { size: 11 } },
              grid: { color: 'rgba(51,65,85,0.5)' },
              border: { display: false },
            },
          },
        },
      });
    },
  };
}
