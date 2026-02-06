/**
 * Charge Points (Stations) page
 */
async function renderChargePoints() {
    document.getElementById('page-title').textContent = 'Станции';

    setPage(`
        <div class="fade-in space-y-4">
            <!-- Header -->
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="relative">
                        <input id="cp-search" type="text" placeholder="Поиск станций..."
                            class="pl-10 pr-4 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 w-64">
                        <svg class="w-4 h-4 text-gray-400 absolute left-3 top-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                    </div>
                    <select id="cp-filter-status" class="border border-gray-300 rounded-lg text-sm px-3 py-2 focus:ring-2 focus:ring-indigo-500">
                        <option value="">Все статусы</option>
                        <option value="Online">Онлайн</option>
                        <option value="Offline">Оффлайн</option>
                        <option value="Unavailable">Недоступна</option>
                    </select>
                </div>
                <button onclick="refreshChargePoints()" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                    Обновить
                </button>
            </div>

            <!-- Table -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div id="cp-table" class="overflow-x-auto">
                    <div class="p-8 text-center text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    document.getElementById('cp-search').addEventListener('input', debounce(filterChargePoints, 300));
    document.getElementById('cp-filter-status').addEventListener('change', filterChargePoints);

    await refreshChargePoints();
}

let allChargePoints = [];
let chargePointStatuses = {};

async function refreshChargePoints() {
    try {
        const [cps, heartbeats] = await Promise.all([
            API.chargePoints.list(),
            API.monitoring.heartbeats().catch(() => []),
        ]);
        allChargePoints = cps || [];
        chargePointStatuses = {};
        (heartbeats || []).forEach(h => {
            chargePointStatuses[h.charge_point_id] = h.status;
        });
        filterChargePoints();
    } catch (e) {
        showToast('Ошибка загрузки станций: ' + e.message, 'error');
    }
}

function filterChargePoints() {
    const search = (document.getElementById('cp-search')?.value || '').toLowerCase();
    const statusFilter = document.getElementById('cp-filter-status')?.value || '';

    let filtered = allChargePoints;

    if (search) {
        filtered = filtered.filter(cp =>
            cp.id.toLowerCase().includes(search) ||
            (cp.vendor || '').toLowerCase().includes(search) ||
            (cp.model || '').toLowerCase().includes(search)
        );
    }

    if (statusFilter) {
        filtered = filtered.filter(cp => {
            const st = chargePointStatuses[cp.id] || cp.status || 'Unknown';
            return st === statusFilter;
        });
    }

    renderChargePointTable(filtered);
}

function renderChargePointTable(cps) {
    const container = document.getElementById('cp-table');
    if (!container) return;

    if (!cps.length) {
        container.innerHTML = '<div class="p-8 text-center text-gray-400">Нет станций</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Производитель</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Модель</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Серийный номер</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Прошивка</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Статус</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Последний heartbeat</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Действия</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${cps.map(cp => {
                    const st = chargePointStatuses[cp.id] || cp.status || 'Unknown';
                    return `
                    <tr class="hover:bg-gray-50 transition">
                        <td class="px-5 py-3">
                            <a href="#/charge-points/${encodeURIComponent(cp.id)}" class="font-medium text-indigo-600 hover:text-indigo-800">${escapeHtml(cp.id)}</a>
                        </td>
                        <td class="px-5 py-3 text-gray-600">${escapeHtml(cp.vendor || '—')}</td>
                        <td class="px-5 py-3 text-gray-600">${escapeHtml(cp.model || '—')}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs font-mono">${escapeHtml(cp.serial_number || '—')}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${escapeHtml(cp.firmware_version || '—')}</td>
                        <td class="px-5 py-3">${statusBadge(st, st)}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${timeAgo(cp.last_heartbeat)}</td>
                        <td class="px-5 py-3">
                            <div class="flex items-center gap-2">
                                <button onclick="sendReset('${escapeHtml(cp.id)}', 'Soft')" title="Soft Reset" class="text-yellow-500 hover:text-yellow-700">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                                </button>
                                <button onclick="sendReset('${escapeHtml(cp.id)}', 'Hard')" title="Hard Reset" class="text-red-500 hover:text-red-700">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
                                </button>
                            </div>
                        </td>
                    </tr>
                    `;
                }).join('')}
            </tbody>
        </table>
    `;
}

async function sendReset(cpId, type) {
    if (!confirm(`Отправить ${type} Reset для ${cpId}?`)) return;
    try {
        await API.commands.reset(cpId, type);
        showToast(`${type} Reset отправлен на ${cpId}`, 'success');
    } catch (e) {
        showToast('Ошибка: ' + e.message, 'error');
    }
}

/** Charge point detail page */
async function renderChargePointDetail({ routeParams }) {
    const cpId = decodeURIComponent(routeParams.id);
    document.getElementById('page-title').textContent = `Станция: ${cpId}`;

    setPage('<div class="p-8 text-center"><div class="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full mx-auto"></div></div>');

    try {
        const [cp, heartbeats, transactions] = await Promise.all([
            API.chargePoints.get(cpId).catch(() => null),
            API.monitoring.heartbeats().catch(() => []),
            API.transactions.list().catch(() => []),
        ]);

        if (!cp) {
            setPage('<div class="p-8 text-center text-gray-500">Станция не найдена</div>');
            return;
        }

        const hb = (heartbeats || []).find(h => h.charge_point_id === cpId);
        const cpStatus = (hb?.status || cp.status || 'Unknown');
        const cpTransactions = (transactions || []).filter(t => t.charge_point_id === cpId);

        setPage(`
            <div class="fade-in space-y-6">
                <div class="flex items-center gap-3">
                    <a href="#/charge-points" class="text-gray-400 hover:text-gray-600">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
                    </a>
                    <h3 class="text-xl font-bold text-gray-900">${escapeHtml(cpId)}</h3>
                    ${statusBadge(cpStatus, cpStatus)}
                </div>

                <!-- Info cards -->
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                        <p class="text-sm text-gray-500 mb-3">Информация</p>
                        <dl class="space-y-2 text-sm">
                            <div class="flex justify-between"><dt class="text-gray-500">Производитель</dt><dd class="font-medium">${escapeHtml(cp.vendor || '—')}</dd></div>
                            <div class="flex justify-between"><dt class="text-gray-500">Модель</dt><dd class="font-medium">${escapeHtml(cp.model || '—')}</dd></div>
                            <div class="flex justify-between"><dt class="text-gray-500">Серийный №</dt><dd class="font-mono text-xs">${escapeHtml(cp.serial_number || '—')}</dd></div>
                            <div class="flex justify-between"><dt class="text-gray-500">Прошивка</dt><dd>${escapeHtml(cp.firmware_version || '—')}</dd></div>
                        </dl>
                    </div>
                    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                        <p class="text-sm text-gray-500 mb-3">Состояние</p>
                        <dl class="space-y-2 text-sm">
                            <div class="flex justify-between"><dt class="text-gray-500">Статус</dt><dd>${statusBadge(cpStatus, cpStatus)}</dd></div>
                            <div class="flex justify-between"><dt class="text-gray-500">Последний heartbeat</dt><dd>${timeAgo(cp.last_heartbeat)}</dd></div>
                            <div class="flex justify-between"><dt class="text-gray-500">Зарегистрирована</dt><dd class="text-xs">${formatDate(cp.registered_at)}</dd></div>
                        </dl>
                    </div>
                    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                        <p class="text-sm text-gray-500 mb-3">Команды</p>
                        <div class="space-y-2">
                            <button onclick="sendReset('${escapeHtml(cpId)}', 'Soft')"
                                class="w-full px-3 py-2 bg-yellow-50 text-yellow-700 text-sm rounded-lg hover:bg-yellow-100 transition">
                                🔄 Soft Reset
                            </button>
                            <button onclick="sendReset('${escapeHtml(cpId)}', 'Hard')"
                                class="w-full px-3 py-2 bg-red-50 text-red-700 text-sm rounded-lg hover:bg-red-100 transition">
                                ⚠️ Hard Reset
                            </button>
                        </div>
                    </div>
                </div>

                <!-- Transactions -->
                <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                    <div class="px-5 py-4 border-b border-gray-100">
                        <h3 class="font-semibold text-gray-800">Транзакции</h3>
                    </div>
                    <div class="overflow-x-auto">
                        ${cpTransactions.length ? `
                            <table class="w-full text-sm">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                                        <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">IdТег</th>
                                        <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Начало</th>
                                        <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Конец</th>
                                        <th class="px-5 py-2.5 text-left text-xs font-medium text-gray-500 uppercase">Статус</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-gray-50">
                                    ${cpTransactions.slice(0, 20).map(tx => `
                                        <tr class="hover:bg-gray-50">
                                            <td class="px-5 py-3 font-medium">#${tx.id}</td>
                                            <td class="px-5 py-3"><code class="text-xs bg-gray-100 px-1.5 py-0.5 rounded">${escapeHtml(tx.id_tag || '—')}</code></td>
                                            <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(tx.started_at)}</td>
                                            <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(tx.stopped_at)}</td>
                                            <td class="px-5 py-3">${tx.status === 'Active' ? statusBadge('charging', 'Активна') : statusBadge('offline', 'Завершена')}</td>
                                        </tr>
                                    `).join('')}
                                </tbody>
                            </table>
                        ` : '<div class="p-5 text-center text-gray-400 text-sm">Нет транзакций</div>'}
                    </div>
                </div>
            </div>
        `);
    } catch (e) {
        setPage(`<div class="p-8 text-center text-red-500">Ошибка: ${escapeHtml(e.message)}</div>`);
    }
}

/** Debounce utility */
function debounce(fn, ms) {
    let timer;
    return (...args) => {
        clearTimeout(timer);
        timer = setTimeout(() => fn(...args), ms);
    };
}
