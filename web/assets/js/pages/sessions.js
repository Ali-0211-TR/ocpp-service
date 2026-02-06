/**
 * Sessions (Transactions) page
 */
async function renderSessions() {
    document.getElementById('page-title').textContent = 'Сессии зарядки';

    setPage(`
        <div class="fade-in space-y-4">
            <div class="flex items-center justify-between flex-wrap gap-3">
                <div class="flex items-center gap-3">
                    <div class="relative">
                        <input id="sess-search" type="text" placeholder="Поиск по станции или тегу..."
                            class="pl-10 pr-4 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 w-64">
                        <svg class="w-4 h-4 text-gray-400 absolute left-3 top-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                    </div>
                    <select id="sess-filter" class="border border-gray-300 rounded-lg text-sm px-3 py-2 focus:ring-2 focus:ring-indigo-500">
                        <option value="">Все</option>
                        <option value="active">Активные</option>
                        <option value="completed">Завершённые</option>
                    </select>
                </div>
                <button onclick="refreshSessions()" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                    Обновить
                </button>
            </div>

            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div id="sess-table" class="overflow-x-auto">
                    <div class="p-8 text-center text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    document.getElementById('sess-search').addEventListener('input', debounce(filterSessions, 300));
    document.getElementById('sess-filter').addEventListener('change', filterSessions);

    await refreshSessions();
}

let allSessions = [];

async function refreshSessions() {
    try {
        allSessions = await API.transactions.list() || [];
        filterSessions();
    } catch (e) {
        showToast('Ошибка загрузки сессий: ' + e.message, 'error');
    }
}

function filterSessions() {
    const search = (document.getElementById('sess-search')?.value || '').toLowerCase();
    const filter = document.getElementById('sess-filter')?.value || '';

    let filtered = allSessions;

    if (search) {
        filtered = filtered.filter(s =>
            (s.charge_point_id || '').toLowerCase().includes(search) ||
            (s.id_tag || '').toLowerCase().includes(search) ||
            String(s.id).includes(search)
        );
    }

    if (filter === 'active') {
        filtered = filtered.filter(s => s.status === 'Active');
    } else if (filter === 'completed') {
        filtered = filtered.filter(s => s.status === 'Completed');
    }

    renderSessionsTable(filtered);
}

function renderSessionsTable(sessions) {
    const container = document.getElementById('sess-table');
    if (!container) return;

    if (!sessions.length) {
        container.innerHTML = '<div class="p-8 text-center text-gray-400">Нет сессий</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Станция</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Коннектор</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">IdТег</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Начало</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Конец</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Энергия</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Стоимость</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Статус</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${sessions.map(s => `
                    <tr class="hover:bg-gray-50 transition">
                        <td class="px-5 py-3 font-medium text-gray-900">#${s.id}</td>
                        <td class="px-5 py-3">
                            <a href="#/charge-points/${encodeURIComponent(s.charge_point_id || '')}" class="text-indigo-600 hover:text-indigo-800">${escapeHtml(s.charge_point_id || '—')}</a>
                        </td>
                        <td class="px-5 py-3 text-gray-500">${s.connector_id || '—'}</td>
                        <td class="px-5 py-3"><code class="text-xs bg-gray-100 px-1.5 py-0.5 rounded">${escapeHtml(s.id_tag || '—')}</code></td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(s.started_at)}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(s.stopped_at)}</td>
                        <td class="px-5 py-3 text-gray-600">${s.energy_consumed_wh != null ? formatEnergy(s.energy_consumed_wh) : '—'}</td>
                        <td class="px-5 py-3 text-gray-600 font-medium">—</td>
                        <td class="px-5 py-3">${s.status === 'Active'
                            ? statusBadge('charging', 'Активна')
                            : statusBadge('offline', 'Завершена')
                        }</td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
    `;
}
