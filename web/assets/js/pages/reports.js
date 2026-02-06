/**
 * Reports page
 */
async function renderReports() {
    document.getElementById('page-title').textContent = 'Отчёты';

    setPage(`
        <div class="fade-in space-y-6">
            <!-- Summary stats -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <p class="text-sm text-gray-500">Всего транзакций</p>
                    <p id="rep-total-tx" class="text-2xl font-bold text-gray-900 mt-1">—</p>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <p class="text-sm text-gray-500">Активных сессий</p>
                    <p id="rep-active" class="text-2xl font-bold text-blue-600 mt-1">—</p>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <p class="text-sm text-gray-500">Общая энергия</p>
                    <p id="rep-energy" class="text-2xl font-bold text-green-600 mt-1">—</p>
                </div>
                <div class="stat-card bg-white rounded-xl shadow-sm border border-gray-100 p-5">
                    <p class="text-sm text-gray-500">Общая выручка</p>
                    <p id="rep-revenue" class="text-2xl font-bold text-purple-600 mt-1">—</p>
                </div>
            </div>

            <!-- Transactions per station -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100">
                    <h3 class="font-semibold text-gray-800">Транзакции по станциям</h3>
                </div>
                <div id="rep-per-station" class="p-5">
                    <div class="text-center text-gray-400 text-sm">Загрузка...</div>
                </div>
            </div>

            <!-- Top IdTags -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100">
                    <h3 class="font-semibold text-gray-800">Топ IdТеги по использованию</h3>
                </div>
                <div id="rep-top-tags" class="p-5">
                    <div class="text-center text-gray-400 text-sm">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    await loadReportData();
}

async function loadReportData() {
    try {
        const transactions = await API.transactions.list().catch(() => []);

        const total = transactions.length;
        const active = transactions.filter(t => t.status === 'Active').length;
        let totalEnergy = 0;
        let totalRevenue = 0;

        const stationMap = {};
        const tagMap = {};

        transactions.forEach(t => {
            if (t.energy_consumed_wh) {
                totalEnergy += t.energy_consumed_wh;
            }

            // Per station
            const cpId = t.charge_point_id || 'Unknown';
            if (!stationMap[cpId]) stationMap[cpId] = { count: 0, energy: 0, revenue: 0 };
            stationMap[cpId].count++;
            if (t.energy_consumed_wh) stationMap[cpId].energy += t.energy_consumed_wh;

            // Per tag
            const tag = t.id_tag || 'Unknown';
            if (!tagMap[tag]) tagMap[tag] = 0;
            tagMap[tag]++;
        });

        document.getElementById('rep-total-tx').textContent = total;
        document.getElementById('rep-active').textContent = active;
        document.getElementById('rep-energy').textContent = formatEnergy(totalEnergy);
        document.getElementById('rep-revenue').textContent = formatMoney(totalRevenue);

        // Per station
        const stationContainer = document.getElementById('rep-per-station');
        const stations = Object.entries(stationMap).sort((a, b) => b[1].count - a[1].count);
        if (stations.length) {
            stationContainer.innerHTML = `
                <div class="space-y-3">
                    ${stations.map(([id, data]) => `
                        <div class="flex items-center gap-4">
                            <a href="#/charge-points/${encodeURIComponent(id)}" class="text-sm font-medium text-indigo-600 hover:text-indigo-800 w-40 truncate">${escapeHtml(id)}</a>
                            <div class="flex-1 bg-gray-100 rounded-full h-4 overflow-hidden">
                                <div class="bg-indigo-500 h-full rounded-full" style="width: ${Math.max(5, (data.count / total) * 100)}%"></div>
                            </div>
                            <span class="text-sm text-gray-500 w-20 text-right">${data.count} сессий</span>
                            <span class="text-sm text-gray-500 w-24 text-right">${formatEnergy(data.energy)}</span>
                        </div>
                    `).join('')}
                </div>
            `;
        } else {
            stationContainer.innerHTML = '<div class="text-center text-gray-400 text-sm">Нет данных</div>';
        }

        // Top tags
        const tagContainer = document.getElementById('rep-top-tags');
        const topTags = Object.entries(tagMap).sort((a, b) => b[1] - a[1]).slice(0, 10);
        if (topTags.length) {
            tagContainer.innerHTML = `
                <div class="space-y-3">
                    ${topTags.map(([tag, count]) => `
                        <div class="flex items-center gap-4">
                            <code class="text-sm bg-gray-100 px-2 py-0.5 rounded font-mono w-44 truncate">${escapeHtml(tag)}</code>
                            <div class="flex-1 bg-gray-100 rounded-full h-4 overflow-hidden">
                                <div class="bg-purple-500 h-full rounded-full" style="width: ${Math.max(5, (count / total) * 100)}%"></div>
                            </div>
                            <span class="text-sm text-gray-500 w-20 text-right">${count} сессий</span>
                        </div>
                    `).join('')}
                </div>
            `;
        } else {
            tagContainer.innerHTML = '<div class="text-center text-gray-400 text-sm">Нет данных</div>';
        }
    } catch (e) {
        console.error('Report load error:', e);
    }
}
