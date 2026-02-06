/**
 * Main application entry point
 * Initializes router, auth check, and WebSocket status
 */
(function () {
    'use strict';

    // ─── Router setup ───────────────────────────
    const router = new Router();

    // Public routes
    router.route('/login', () => {
        showLoginPage();
    });

    // Protected routes
    router.route('/dashboard', async () => {
        await renderDashboard();
    });

    router.route('/charge-points', async () => {
        await renderChargePoints();
    });

    router.route('/charge-points/:id', async (ctx) => {
        await renderChargePointDetail(ctx);
    });

    router.route('/tags', async () => {
        await renderTags();
    });

    router.route('/sessions', async () => {
        await renderSessions();
    });

    router.route('/tariffs', async () => {
        await renderTariffs();
    });

    router.route('/reports', async () => {
        await renderReports();
    });

    router.route('/users', async () => {
        await renderUsers();
    });

    router.route('/api-keys', async () => {
        await renderApiKeys();
    });

    router.route('/settings', async () => {
        await renderSettings();
    });

    // ─── WebSocket status indicator ─────────────
    function updateWsStatus() {
        const dot = document.getElementById('ws-dot');
        const text = document.getElementById('ws-text');
        if (!dot || !text) return;

        if (WS.ws && WS.ws.readyState === WebSocket.OPEN) {
            dot.className = 'w-2 h-2 rounded-full bg-green-500 pulse-dot';
            text.textContent = 'Онлайн';
            text.className = 'text-sm text-green-600';
        } else if (WS.ws && WS.ws.readyState === WebSocket.CONNECTING) {
            dot.className = 'w-2 h-2 rounded-full bg-yellow-400';
            text.textContent = 'Подключение...';
            text.className = 'text-sm text-yellow-600';
        } else {
            dot.className = 'w-2 h-2 rounded-full bg-gray-300';
            text.textContent = 'Отключено';
            text.className = 'text-sm text-gray-500';
        }
    }

    // Monitor WS status
    setInterval(updateWsStatus, 2000);

    // WS event listeners for toasts
    WS.on('ChargePointConnected', (ev) => {
        showToast(`🟢 Станция подключена: ${ev.data?.charge_point_id || ''}`, 'success');
    });

    WS.on('ChargePointDisconnected', (ev) => {
        showToast(`🔴 Станция отключена: ${ev.data?.charge_point_id || ''}`, 'warning');
    });

    WS.on('TransactionStarted', (ev) => {
        showToast(`▶️ Начата зарядка: ${ev.data?.charge_point_id || ''}`, 'info');
    });

    WS.on('TransactionStopped', (ev) => {
        showToast(`⏹️ Зарядка завершена: ${ev.data?.charge_point_id || ''}`, 'info');
    });

    WS.on('Error', (ev) => {
        showToast(`❌ Ошибка: ${ev.data?.message || ''}`, 'error');
    });

    // ─── Notification counter ───────────────────
    let notifCount = 0;
    WS.on('*', () => {
        notifCount++;
        const badge = document.getElementById('notif-badge');
        if (badge) {
            badge.textContent = notifCount > 99 ? '99+' : notifCount;
            badge.classList.remove('hidden');
        }
    });

    // Reset notification count on click
    document.getElementById('notif-btn')?.addEventListener('click', () => {
        notifCount = 0;
        const badge = document.getElementById('notif-badge');
        if (badge) badge.classList.add('hidden');
    });

    // ─── Init ───────────────────────────────────
    initLoginPage();

    // Check existing token
    const token = localStorage.getItem('token');
    if (token) {
        API.token = token;
        // Verify token is still valid
        API.auth.me().then(() => {
            showMainLayout();
            router.start();
        }).catch(() => {
            API.token = null;
            localStorage.removeItem('token');
            showLoginPage();
            if (!location.hash || location.hash === '#/') {
                location.hash = '#/login';
            }
            router.start();
        });
    } else {
        showLoginPage();
        location.hash = '#/login';
        router.start();
    }
})();
