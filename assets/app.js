function dashboard() {
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
      setInterval(() => this.fetchMonitors(), 30_000);
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
      };
      return icons[protocol] || 'fa-solid fa-circle-question';
    },
  };
}
