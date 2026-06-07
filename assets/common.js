// Shared front-end helpers, loaded before app.js / monitor.js. Kept tiny and
// dependency-free so both the dashboard and the detail page can reuse it.
window.RP = window.RP || {};

// Returns true when a traceroute monitor is down because it couldn't open a raw
// socket — i.e. it needs CAP_NET_RAW / elevated privileges.
// Coupled to probe/traceroute.rs reason strings; update both together.
window.RP.isTraceUnavailable = function (protocol, status, failureReason) {
  if (protocol !== 'traceroute' || status !== 'down') return false;
  return failureReason === 'privilege_error'
      || failureReason === 'socket_error'
      || failureReason === 'join_error';
};

// "x ago" from an ISO timestamp. (Was duplicated verbatim in both pages.)
window.RP.formatRelative = function (iso) {
  const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (diff < 10) return 'just now';
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
};

// Subscribe to the live status SSE stream. Wraps the EventSource creation + JSON
// parsing both pages shared; callers supply their own update/open/error handling
// (and their own poll fallback). Returns the EventSource, or null when SSE is
// unavailable so the caller can fall back to polling.
window.RP.subscribeStatus = function (opts) {
  const onUpdate = opts.onUpdate || function () {};
  if (!('EventSource' in window)) return null;
  let es;
  try {
    es = new EventSource('/api/monitors/stream');
  } catch (e) {
    if (opts.onError) opts.onError(e);
    return null;
  }
  es.onmessage = (ev) => {
    let u;
    try { u = JSON.parse(ev.data); } catch (e) { return; }
    onUpdate(u);
  };
  if (opts.onOpen) es.onopen = opts.onOpen;
  if (opts.onError) es.onerror = opts.onError;
  return es;
};
