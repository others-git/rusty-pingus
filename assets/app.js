function dashboard() {
  // Held in a closure, deliberately OFF the reactive component object: Alpine
  // would otherwise proxy the EventSource (host object), and reactive proxies
  // over host objects are a known footgun here.
  let eventSource = null;

  return {
    monitors: [],
    loading: true,
    lastUpdated: null,
    pendingDelete: null,
    showAddModal: false,
    submitting: false,
    form: {
      protocol: 'http',
      name: '',
      url: '',
      host: '',
      port: 443,
      method: 'GET',
      expected_status: null,
      gateway: '',
      upstream: '1.1.1.1',
      interval_ms: 60000,
      timeout_ms: 10000,
    },
    formErrors: {},

    get upCount() {
      return this.monitors.filter(m => m.status === 'up').length;
    },
    get downCount() {
      return this.monitors.filter(m => m.status === 'down').length;
    },

    async init() {
      await this.fetchMonitors();
      // Periodic poll: fallback if SSE is unavailable/drops, and the source of
      // truth for list membership (add/remove) and 24h uptime.
      setInterval(() => this.fetchMonitors(), 30_000);
      // Live updates pushed from the server as probes complete.
      this.connectStream();
      // Best-effort teardown so we don't leak the connection on navigation.
      window.addEventListener('beforeunload', () => { if (eventSource) eventSource.close(); });
    },

    connectStream() {
      if (!('EventSource' in window)) return; // no SSE support → poll-only
      try {
        eventSource = new EventSource('/api/monitors/stream');
      } catch (e) {
        console.warn('SSE unavailable, relying on poll', e);
        return;
      }
      eventSource.onmessage = (ev) => {
        let u;
        try { u = JSON.parse(ev.data); } catch (e) { return; }
        const m = this.monitors.find(x => x.name === u.name);
        if (!m) return; // not in the list yet — the next poll will add it
        // Update the card's live fields in place; up/down counts are getters
        // and recompute automatically. 24h uptime stays on the poll.
        m.status = u.status;
        m.response_time_ms = u.response_time_ms;
        m.last_checked_at = u.last_checked_at;
        m.failure_reason = u.failure_reason;
        m.detail = u.detail;
        this.lastUpdated = this.formatRelative(new Date().toISOString());
      };
      // On error the browser auto-reconnects; the poll covers any gap meanwhile.
      eventSource.onerror = () => {};
    },

    async fetchMonitors() {
      try {
        const res = await fetch('/api/monitors');
        this.monitors = await res.json();
        this.lastUpdated = this.formatRelative(new Date().toISOString());
      } catch (e) {
        console.error('Poll failed', e);
      } finally {
        this.loading = false;
      }
    },

    async deleteMonitor(name) {
      this.pendingDelete = null;
      try {
        const res = await fetch(`/api/monitors/${encodeURIComponent(name)}`, { method: 'DELETE' });
        if (res.ok) {
          this.monitors = this.monitors.filter(m => m.name !== name);
        }
      } catch (e) {
        console.error('Delete failed', e);
      }
    },

    async submitMonitor() {
      this.formErrors = {};
      this.submitting = true;

      const body = {
        protocol: this.form.protocol,
        name: this.form.name,
        interval_ms: this.form.interval_ms,
        timeout_ms: this.form.timeout_ms,
      };

      if (this.form.protocol === 'http') {
        body.url = this.form.url;
        body.method = this.form.method;
        if (this.form.expected_status) body.expected_status = this.form.expected_status;
      } else if (this.form.protocol === 'publicip') {
        // URL is optional; omit it to use the default service with fallback.
        if (this.form.url && this.form.url.trim()) body.url = this.form.url.trim();
      } else if (this.form.protocol === 'border') {
        if (this.form.gateway && this.form.gateway.trim()) body.gateway = this.form.gateway.trim();
        body.upstream = (this.form.upstream && this.form.upstream.trim()) || '1.1.1.1';
      } else {
        body.host = this.form.host;
        if (this.form.protocol === 'tcp') body.port = this.form.port;
      }

      try {
        const res = await fetch('/api/monitors', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });

        if (res.status === 201) {
          // Success — refresh monitors and close modal
          await this.fetchMonitors();
          this.showAddModal = false;
          this.resetForm();
        } else if (res.status === 422) {
          const data = await res.json();
          this.mapErrors(data.errors || []);
        } else {
          this.formErrors._general = 'Unexpected error. Please try again.';
        }
      } catch (e) {
        this.formErrors._general = 'Network error. Is the server running?';
      } finally {
        this.submitting = false;
      }
    },

    mapErrors(errors) {
      for (const msg of errors) {
        if (msg.includes('name')) this.formErrors.name = msg;
        else if (msg.includes('url')) this.formErrors.url = msg;
        else if (msg.includes('gateway')) this.formErrors.gateway = msg;
        else if (msg.includes('upstream')) this.formErrors.upstream = msg;
        else if (msg.includes('host')) this.formErrors.host = msg;
        else if (msg.includes('port')) this.formErrors.port = msg;
        else if (msg.includes('interval')) this.formErrors.interval_ms = msg;
        else if (msg.includes('timeout')) this.formErrors.timeout_ms = msg;
        else this.formErrors._general = msg;
      }
    },

    resetForm() {
      this.form = {
        protocol: 'http',
        name: '',
        url: '',
        host: '',
        port: 443,
        method: 'GET',
        expected_status: null,
        gateway: '',
        upstream: '1.1.1.1',
        interval_ms: 60000,
        timeout_ms: 10000,
      };
      this.formErrors = {};
    },

    formatRelative(iso) {
      const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
      if (diff < 10) return 'just now';
      if (diff < 60) return `${diff}s ago`;
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return `${Math.floor(diff / 86400)}d ago`;
    },

    protocolIcon(protocol) {
      const icons = {
        http: 'fa-solid fa-globe',
        tcp: 'fa-solid fa-network-wired',
        icmp: 'fa-solid fa-satellite-dish',
        publicip: 'fa-solid fa-location-crosshairs',
        border: 'fa-solid fa-shield-halved',
      };
      return icons[protocol] || 'fa-solid fa-circle-question';
    },

    protocolLabel(protocol) {
      const labels = { publicip: 'Public IP', border: 'Border' };
      return labels[protocol] || protocol.toUpperCase();
    },

    // Split a probe's detail for display. A border detail's trailing RTT group
    // "(gw X, upstream Y)" is broken onto its own lines so it stays inside the
    // card; other details render as a single line.
    formatDetail(detail) {
      if (!detail) return [];
      const m = detail.match(/^(.*?)\s*\(gw\s*(.*?),\s*upstream\s*(.*?)\)\s*$/);
      if (!m) return [detail];
      const lines = [];
      if (m[1].trim()) lines.push(m[1].trim());
      lines.push(`gw ${m[2].trim()}`);
      lines.push(`upstream ${m[3].trim()}`);
      return lines;
    },
  };
}
