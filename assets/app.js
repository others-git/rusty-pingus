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
    // When set, the modal is in edit mode for this monitor id. editingOriginal
    // holds its full stored config so fields the form doesn't expose (HTTP
    // headers/body, ICMP count, border isp_gateway, paused state) survive a save.
    editingId: null,
    editingOriginal: null,
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
      max_hops: 30,
      queries_per_hop: 3,
      retention_hours: null,
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
      // Shared subscribe helper; the poll (init) covers any gap if SSE is down.
      eventSource = window.RP.subscribeStatus({
        onUpdate: (u) => {
          const m = this.monitors.find(x => x.id === u.id);
          if (!m) return; // not in the list yet — the next poll will add it
          // Update the card's live fields in place; up/down counts are getters
          // and recompute automatically. 24h uptime stays on the poll.
          m.status = u.status;
          m.response_time_ms = u.response_time_ms;
          m.last_checked_at = u.last_checked_at;
          m.failure_reason = u.failure_reason;
          m.detail = u.detail;
          this.lastUpdated = this.formatRelative(new Date().toISOString());
        },
        onError: () => {}, // browser auto-reconnects; the poll covers the gap
      });
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

    async toggleMonitor(m) {
      const next = !m.enabled;
      m.enabled = next; // optimistic; revert on failure
      try {
        const res = await fetch(`/api/monitors/${m.id}/enabled`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ enabled: next }),
        });
        if (!res.ok) { m.enabled = !next; return; }
      } catch (e) {
        m.enabled = !next; // network error → undo
      }
    },

    async deleteMonitor(id) {
      this.pendingDelete = null;
      try {
        const res = await fetch(`/api/monitors/${id}`, { method: 'DELETE' });
        if (res.ok) {
          this.monitors = this.monitors.filter(m => m.id !== id);
        }
      } catch (e) {
        console.error('Delete failed', e);
      }
    },

    // Build the monitor payload from the current form. Authoritative for every
    // field the form manages (present/absent decides set/cleared); fields the UI
    // doesn't expose are layered on separately when editing (see submitMonitor).
    buildBody() {
      const body = {
        protocol: this.form.protocol,
        name: this.form.name,
        interval_ms: this.form.interval_ms,
        timeout_ms: this.form.timeout_ms,
      };

      // Per-monitor retention (hours) applies to all protocols; omit when blank.
      if (this.form.retention_hours > 0) body.retention_hours = this.form.retention_hours;

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
      } else if (this.form.protocol === 'traceroute') {
        body.host = this.form.host;
        body.max_hops = this.form.max_hops;
        body.queries_per_hop = this.form.queries_per_hop;
      } else {
        body.host = this.form.host;
        if (this.form.protocol === 'tcp') body.port = this.form.port;
      }
      return body;
    },

    async submitMonitor() {
      this.formErrors = {};
      this.submitting = true;

      const body = this.buildBody();

      if (this.editingId != null) {
        // Carry over fields the form doesn't expose so an edit never silently
        // drops them, and keep a paused monitor paused.
        const o = this.editingOriginal || {};
        if (o.headers) body.headers = o.headers;
        if (o.body != null) body.body = o.body;
        if (this.form.protocol === 'icmp' && o.count != null) body.count = o.count;
        if (this.form.protocol === 'border' && o.isp_gateway != null) body.isp_gateway = o.isp_gateway;
        if (o.enabled === false) body.enabled = false;
      }

      const editing = this.editingId != null;
      const url = editing ? `/api/monitors/${this.editingId}` : '/api/monitors';

      try {
        const res = await fetch(url, {
          method: editing ? 'PUT' : 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });

        if (res.status === 200 || res.status === 201) {
          // Success — refresh monitors and close modal
          await this.fetchMonitors();
          this.closeModal();
        } else if (res.status === 422) {
          const data = await res.json();
          this.mapErrors(data.errors || []);
        } else if (res.status === 404) {
          this.formErrors._general = 'Monitor not found — it may have been deleted.';
        } else {
          this.formErrors._general = 'Unexpected error. Please try again.';
        }
      } catch (e) {
        this.formErrors._general = 'Network error. Is the server running?';
      } finally {
        this.submitting = false;
      }
    },

    openAdd() {
      this.resetForm();
      this.editingId = null;
      this.editingOriginal = null;
      this.showAddModal = true;
    },

    async openEdit(m) {
      this.formErrors = {};
      try {
        const res = await fetch('/api/monitors/config');
        const configs = await res.json();
        const cfg = configs.find(c => c.id === m.id);
        if (!cfg) { console.error('Config not found for', m.id); return; }

        this.editingOriginal = cfg;
        this.editingId = cfg.id;
        // Populate the form from the stored config, defaulting fields this
        // protocol doesn't use so the inputs stay well-formed.
        this.form = {
          protocol: cfg.protocol,
          name: cfg.name,
          url: cfg.url || '',
          host: cfg.host || '',
          port: cfg.port || 443,
          method: cfg.method || 'GET',
          expected_status: cfg.expected_status ?? null,
          gateway: cfg.gateway || '',
          upstream: cfg.upstream || '1.1.1.1',
          max_hops: cfg.max_hops || 30,
          queries_per_hop: cfg.queries_per_hop || 3,
          retention_hours: cfg.retention_hours ?? null,
          interval_ms: cfg.interval_ms ?? 60000,
          timeout_ms: cfg.timeout_ms ?? 10000,
        };
        this.showAddModal = true;
      } catch (e) {
        console.error('Failed to load monitor config', e);
      }
    },

    closeModal() {
      this.showAddModal = false;
      this.editingId = null;
      this.editingOriginal = null;
      this.resetForm();
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
        max_hops: 30,
        queries_per_hop: 3,
        retention_hours: null,
        interval_ms: 60000,
        timeout_ms: 10000,
      };
      this.formErrors = {};
    },

    formatRelative(iso) {
      return window.RP.formatRelative(iso);
    },

    isTraceUnavailable(m) {
      return window.RP.isTraceUnavailable(m.protocol, m.status, m.failure_reason);
    },

    protocolIcon(protocol) {
      const icons = {
        http: 'fa-solid fa-globe',
        tcp: 'fa-solid fa-network-wired',
        icmp: 'fa-solid fa-satellite-dish',
        publicip: 'fa-solid fa-location-crosshairs',
        border: 'fa-solid fa-shield-halved',
        traceroute: 'fa-solid fa-route',
      };
      return icons[protocol] || 'fa-solid fa-circle-question';
    },

    protocolLabel(protocol) {
      const labels = { publicip: 'Public IP', border: 'Border', traceroute: 'Trace' };
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
