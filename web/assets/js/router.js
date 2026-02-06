/**
 * Simple hash-based SPA router
 */
class Router {
    constructor() {
        this.routes = {};
        this.currentPage = null;
        window.addEventListener('hashchange', () => this.navigate());
    }

    /** Register a route */
    route(path, handler) {
        this.routes[path] = handler;
        return this;
    }

    /** Navigate to current hash */
    async navigate() {
        const hash = location.hash.slice(1) || '/dashboard';
        const [path, query] = hash.split('?');
        const params = new URLSearchParams(query || '');

        // Check auth (except login page)
        if (path !== '/login' && !API.token) {
            location.hash = '#/login';
            return;
        }

        // If logged in and going to login, redirect to dashboard
        if (path === '/login' && API.token) {
            location.hash = '#/dashboard';
            return;
        }

        // Find matching route
        let handler = this.routes[path];
        let routeParams = {};

        if (!handler) {
            // Try pattern matching (e.g., /charge-points/:id)
            for (const [pattern, h] of Object.entries(this.routes)) {
                const match = this.matchRoute(pattern, path);
                if (match) {
                    handler = h;
                    routeParams = match;
                    break;
                }
            }
        }

        if (!handler) {
            handler = this.routes['/404'] || (() => {
                document.getElementById('page-content').innerHTML = '<div class="p-8 text-center text-gray-500">Page not found</div>';
            });
        }

        // Update active sidebar link
        this.updateSidebar(path);
        this.currentPage = path;

        try {
            await handler({ path, params, routeParams });
        } catch (e) {
            console.error('Route error:', e);
        }
    }

    /** Pattern match route like /charge-points/:id */
    matchRoute(pattern, path) {
        const patternParts = pattern.split('/');
        const pathParts = path.split('/');
        if (patternParts.length !== pathParts.length) return null;

        const params = {};
        for (let i = 0; i < patternParts.length; i++) {
            if (patternParts[i].startsWith(':')) {
                params[patternParts[i].slice(1)] = pathParts[i];
            } else if (patternParts[i] !== pathParts[i]) {
                return null;
            }
        }
        return params;
    }

    /** Update active sidebar link */
    updateSidebar(path) {
        document.querySelectorAll('[data-nav]').forEach(el => {
            const navPath = el.getAttribute('data-nav');
            if (path.startsWith(navPath)) {
                el.classList.add('bg-indigo-700', 'text-white');
                el.classList.remove('text-indigo-100', 'hover:bg-indigo-600');
            } else {
                el.classList.remove('bg-indigo-700', 'text-white');
                el.classList.add('text-indigo-100', 'hover:bg-indigo-600');
            }
        });
    }

    /** Start the router */
    start() {
        this.navigate();
    }
}

// ─── Utility functions ─────────────────────────

/** Format date to locale string */
function formatDate(dateStr) {
    if (!dateStr) return '—';
    return new Date(dateStr).toLocaleString('ru-RU');
}

/** Format relative time */
function timeAgo(dateStr) {
    if (!dateStr) return 'никогда';
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return `${Math.floor(diff)} сек назад`;
    if (diff < 3600) return `${Math.floor(diff / 60)} мин назад`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} ч назад`;
    return `${Math.floor(diff / 86400)} д назад`;
}

/** Status badge HTML */
function statusBadge(status, text) {
    const colors = {
        online: 'bg-green-100 text-green-800',
        available: 'bg-green-100 text-green-800',
        accepted: 'bg-green-100 text-green-800',
        charging: 'bg-blue-100 text-blue-800',
        preparing: 'bg-yellow-100 text-yellow-800',
        offline: 'bg-gray-100 text-gray-800',
        unavailable: 'bg-red-100 text-red-800',
        faulted: 'bg-red-100 text-red-800',
        blocked: 'bg-red-100 text-red-800',
        unknown: 'bg-gray-100 text-gray-500',
    };
    const key = (status || 'unknown').toLowerCase();
    const cls = colors[key] || colors.unknown;
    return `<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${cls}">${text || status}</span>`;
}

/** Connector status icon */
function connectorIcon(status) {
    const icons = {
        Available: '🟢',
        Charging: '⚡',
        Preparing: '🟡',
        SuspendedEV: '⏸️',
        SuspendedEVSE: '⏸️',
        Finishing: '🏁',
        Reserved: '📌',
        Unavailable: '🔴',
        Faulted: '❌',
    };
    return icons[status] || '⚪';
}

/** Format energy in kWh */
function formatEnergy(wh) {
    if (!wh && wh !== 0) return '—';
    return (wh / 1000).toFixed(2) + ' kWh';
}

/** Format money */
function formatMoney(amount, currency = 'UZS') {
    if (!amount && amount !== 0) return '—';
    return new Intl.NumberFormat('ru-RU', { style: 'currency', currency }).format(amount / 100);
}

/** Escape HTML */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/** Set page content */
function setPage(html) {
    document.getElementById('page-content').innerHTML = html;
}

/** Show toast notification */
function showToast(message, type = 'info') {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const colors = {
        success: 'bg-green-500',
        error: 'bg-red-500',
        warning: 'bg-yellow-500',
        info: 'bg-indigo-500',
    };

    const toast = document.createElement('div');
    toast.className = `${colors[type] || colors.info} text-white px-4 py-3 rounded-lg shadow-lg transform transition-all duration-300 translate-y-2 opacity-0`;
    toast.innerHTML = `
        <div class="flex items-center gap-2">
            <span class="text-sm font-medium">${escapeHtml(message)}</span>
        </div>
    `;
    container.appendChild(toast);

    requestAnimationFrame(() => {
        toast.classList.remove('translate-y-2', 'opacity-0');
    });

    setTimeout(() => {
        toast.classList.add('translate-y-2', 'opacity-0');
        setTimeout(() => toast.remove(), 300);
    }, 4000);
}
