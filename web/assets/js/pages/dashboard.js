/**
 * Dashboard page
 */
async function renderDashboard() {
    document.getElementById('page-title').textContent = 'Дашборд';

    setPage(`
        <div class="fade-in space-y-6">
            <!-- Stats cards -->
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-500">Всего станций</p>
                            <p id="stat-total" class="text-2xl font-bold text-gray-900 mt-1">—</p>
                        </div>
                        <div class="w-12 h-12 bg-indigo-50 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-500">Онлайн</p>
                            <p id="stat-online" class="text-2xl font-bold text-green-600 mt-1">—</p>
                        </div>
                        <div class="w-12 h-12 bg-green-50 rounded-xl flex items-center justify-center">
                            <div class="w-3 h-3 bg-green-500 rounded-full pulse-dot"></div>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-500">Активные сессии</p>
                            <p id="stat-sessions" class="text-2xl font-bold text-blue-600 mt-1">—</p>
                        </div>
                        <div class="w-12 h-12 bg-blue-50 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-500">IdТеги</p>
                            <p id="stat-tags" class="text-2xl font-bold text-purple-600 mt-1">—</p>
                        </div>
                        <div class="w-12 h-12 bg-purple-50 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/></svg>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Two columns -->
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <!-- Station list -->
                <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                    <div class="px-5 py-4 border-b border-gray-100 flex items-center justify-between">
                        <h3 class="font-semibold text-gray-800">Станции</h3>
                        <a href="#/charge-points" class="text-sm text-indigo-600 hover:text-indigo-800">Все →</a>
                    </div>
                    <div id="dash-stations" class="divide-y divide-gray-50 max-h-80 overflow-y-auto">
                        <div class="p-5 text-center text-gray-400 text-sm">Загрузка...</div>
                    </div>
                </div>

                <!-- Live events -->
                <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                    <div class="px-5 py-4 border-b border-gray-100 flex items-center justify-between">
                        <h3 class="font-semibold text-gray-800">Последние события</h3>
                        <button onclick="clearEvents()" class="text-sm text-gray-400 hover:text-gray-600">Очистить</button>
                    </div>
                    <div id="dash-events" class="divide-y divide-gray-50 max-h-80 overflow-y-auto">
                        <div class="p-5 text-center text-gray-400 text-sm">Ожидание событий...</div>
                    </div>
                </div>
            </div>

            <!-- Recent transactions -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100 flex items-center justify-between">
                    <h3 class="font-semibold text-gray-800">Последние транзакции</h3>
                    <a href="#/sessions" class="text-sm text-indigo-600 hover:text-indigo-800">Все →</a>
                </div>
                <div id="dash-transactions" class="overflow-x-auto">
                    <div class="p-5 text-center text-gray-400 text-sm">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    // Load data
    await loadDashboardData();

    // Subscribe to real-time events
    setupDashboardWS();
}

/** Load all dashboard stats */
async function loadDashboardData() {
    try {
        const [chargePoints, stats, heartbeats, tags, transactions] = await Promise.all([
            API.chargePoints.list().catch(() => []),
            API.monitoring.stats().catch(() => ({})),
            API.monitoring.heartbeats().catch(() => []),
            API.idTags.list().catch(() => []),
            API.transactions.list().catch(() => []),
        ]);

        // Build status map from heartbeats
        const statusMap = {};
        (heartbeats || []).forEach(h => { statusMap[h.charge_point_id] = h.status; });

        // Stats
        document.getElementById('stat-total').textContent = (chargePoints || []).length || 0;
        document.getElementById('stat-online').textContent = stats.online || 0;
        const activeSessions = (transactions || []).filter(t => !t.stopped_at && t.status === 'Active').length;
        document.getElementById('stat-sessions').textContent = activeSessions;
        document.getElementById('stat-tags').textContent = (tags || []).length || 0;

        // Nav badge
        const badge = document.getElementById('nav-cp-count');
        if (badge && (chargePoints || []).length) {
            badge.textContent = chargePoints.length;
            badge.classList.remove('hidden');
        }

        // Stations list
        renderDashStations(chargePoints || [], statusMap);

        // Transactions table
        renderDashTransactions(transactions || []);
    } catch (e) {
        console.error('Dashboard load error:', e);
    }
}

/** Render station mini-list */
function renderDashStations(chargePoints, statusMap) {
    const container = document.getElementById('dash-stations');
    if (!container) return;

    if (!chargePoints.length) {
        container.innerHTML = '<div class="p-5 text-center text-gray-400 text-sm">Нет зарегистрированных станций</div>';
        return;
    }

    container.innerHTML = chargePoints.slice(0, 8).map(cp => {
        const st = statusMap[cp.id] || cp.status || 'Unknown';
        return `
            <a href="#/charge-points/${cp.id}" class="flex items-center gap-3 px-5 py-3 hover:bg-gray-50 transition">
                <div class="w-10 h-10 bg-gray-100 rounded-lg flex items-center justify-center text-lg">
                    ⚡
                </div>
                <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-gray-900 truncate">${escapeHtml(cp.id)}</p>
                    <p class="text-xs text-gray-500">${cp.vendor || '—'} ${cp.model || ''}</p>
                </div>
                ${statusBadge(st, st)}
            </a>
        `;
    }).join('');
}

/** Render last transactions */
function renderDashTransactions(transactions) {
    const container = document.getElementById('dash-transactions');
    if (!container) return;

    const recent = transactions.slice(0, 10);
    if (!recent.length) {
        container.innerHTML = '<div class="p-5 text-center text-gray-400 text-sm">Нет транзакций</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                    <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Станция</th>
                    <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">IdТег</th>
                    <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Начало</th>
                    <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Статус</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${recent.map(tx => `
                    <tr class="hover:bg-gray-50">
                        <td class="px-5 py-3 font-medium text-gray-900">#${tx.id}</td>
                        <td class="px-5 py-3 text-gray-600">${escapeHtml(tx.charge_point_id || '—')}</td>
                        <td class="px-5 py-3"><code class="text-xs bg-gray-100 px-1.5 py-0.5 rounded">${escapeHtml(tx.id_tag || '—')}</code></td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(tx.started_at)}</td>
                        <td class="px-5 py-3">${tx.status === 'Active'
                            ? statusBadge('charging', 'Активна')
                            : statusBadge('offline', 'Завершена')
                        }</td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
    `;
}

/** Live events from WebSocket */
const dashEvents = [];

function setupDashboardWS() {
    WS.on('*', onDashboardEvent);
}

function cleanupDashboardWS() {
    WS.off('*', onDashboardEvent);
}

function onDashboardEvent(event) {
    dashEvents.unshift(event);
    if (dashEvents.length > 50) dashEvents.pop();

    const container = document.getElementById('dash-events');
    if (!container) {
        cleanupDashboardWS();
        return;
    }

    renderDashEvents();

    // Refresh stats on significant events
    if (['ChargePointConnected', 'ChargePointDisconnected', 'TransactionStarted', 'TransactionStopped'].includes(event.event_type)) {
        loadDashboardData();
    }
}

function renderDashEvents() {
    const container = document.getElementById('dash-events');
    if (!container) return;

    if (!dashEvents.length) {
        container.innerHTML = '<div class="p-5 text-center text-gray-400 text-sm">Ожидание событий...</div>';
        return;
    }

    const icons = {
        ChargePointConnected: '🟢',
        ChargePointDisconnected: '🔴',
        ChargePointStatusChanged: '🔄',
        ConnectorStatusChanged: '🔌',
        TransactionStarted: '▶️',
        TransactionStopped: '⏹️',
        MeterValuesReceived: '📊',
        HeartbeatReceived: '💓',
        AuthorizationResult: '🔑',
        BootNotification: '🚀',
        Error: '❌',
    };

    container.innerHTML = dashEvents.slice(0, 20).map(ev => `
        <div class="flex items-start gap-3 px-5 py-3">
            <span class="text-lg mt-0.5">${icons[ev.event_type] || '📌'}</span>
            <div class="flex-1 min-w-0">
                <p class="text-sm text-gray-900">${escapeHtml(ev.event_type)}</p>
                <p class="text-xs text-gray-500">${ev.data?.charge_point_id || ''} ${ev.data?.status || ''}</p>
            </div>
            <span class="text-xs text-gray-400 whitespace-nowrap">${timeAgo(ev.timestamp)}</span>
        </div>
    `).join('');
}

function clearEvents() {
    dashEvents.length = 0;
    renderDashEvents();
}
