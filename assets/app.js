function dashboard() {
  return {
    monitors: [],
    loading: true,
    lastUpdated: null,

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
