/**
 * Tariffs management page
 */
async function renderTariffs() {
    document.getElementById('page-title').textContent = 'Тарифы';

    setPage(`
        <div class="fade-in space-y-4">
            <div class="flex items-center justify-between">
                <p class="text-sm text-gray-500">Управление тарифами на зарядку</p>
                <button onclick="showAddTariffModal()" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    Добавить тариф
                </button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div id="tariffs-table" class="overflow-x-auto">
                    <div class="p-8 text-center text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    await refreshTariffs();
}

let allTariffs = [];

async function refreshTariffs() {
    try {
        allTariffs = await API.tariffs.list() || [];
        renderTariffsTable(allTariffs);
    } catch (e) {
        showToast('Ошибка загрузки тарифов: ' + e.message, 'error');
    }
}

function renderTariffsTable(tariffs) {
    const container = document.getElementById('tariffs-table');
    if (!container) return;

    if (!tariffs.length) {
        container.innerHTML = '<div class="p-8 text-center text-gray-400">Нет тарифов</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Название</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Цена за кВт·ч</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Валюта</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Активен</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Создан</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Действия</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${tariffs.map(t => `
                    <tr class="hover:bg-gray-50 transition">
                        <td class="px-5 py-3 font-medium text-gray-900">${t.id}</td>
                        <td class="px-5 py-3 text-gray-700">${escapeHtml(t.name || '—')}</td>
                        <td class="px-5 py-3 text-gray-700 font-mono">${t.price_per_kwh != null ? (t.price_per_kwh / 100).toFixed(2) : '—'}</td>
                        <td class="px-5 py-3 text-gray-500">${escapeHtml(t.currency || 'UZS')}</td>
                        <td class="px-5 py-3">${t.is_active !== false
                            ? '<span class="text-green-500">✓</span>'
                            : '<span class="text-gray-400">✗</span>'
                        }</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(t.created_at)}</td>
                        <td class="px-5 py-3">
                            <button onclick="deleteTariff(${t.id})" class="text-red-500 hover:text-red-700" title="Удалить">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                            </button>
                        </td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
    `;
}

function showAddTariffModal() {
    showModal(`
        <div class="p-6">
            <h3 class="text-lg font-semibold text-gray-900 mb-4">Добавить тариф</h3>
            <form id="add-tariff-form" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Название *</label>
                    <input type="text" id="new-tariff-name" required
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="Стандартный тариф">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Тип тарифа *</label>
                    <select id="new-tariff-type" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        <option value="PerKwh">За кВт·ч</option>
                        <option value="PerMinute">За минуту</option>
                        <option value="Flat">Фиксированный</option>
                        <option value="Combined">Комбинированный</option>
                    </select>
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Цена за кВт·ч (тийин) *</label>
                        <input type="number" id="new-tariff-price" required min="0" value="0"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                            placeholder="150000">
                        <p class="text-xs text-gray-400 mt-1">150000 = 1500.00 UZS</p>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Цена за минуту (тийин) *</label>
                        <input type="number" id="new-tariff-price-per-minute" required min="0" value="0"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                            placeholder="5000">
                    </div>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Плата за сессию (тийин) *</label>
                    <input type="number" id="new-tariff-session-fee" required min="0" value="0"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="0">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Валюта *</label>
                    <select id="new-tariff-currency" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        <option value="UZS">UZS</option>
                        <option value="USD">USD</option>
                        <option value="EUR">EUR</option>
                        <option value="RUB">RUB</option>
                    </select>
                </div>
                <div class="flex justify-end gap-3 pt-2">
                    <button type="button" onclick="hideModal()" class="px-4 py-2 text-gray-700 text-sm font-medium rounded-lg hover:bg-gray-100 transition">Отмена</button>
                    <button type="submit" class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">Создать</button>
                </div>
            </form>
        </div>
    `);

    document.getElementById('add-tariff-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        try {
            await API.tariffs.create({
                name: document.getElementById('new-tariff-name').value,
                tariff_type: document.getElementById('new-tariff-type').value,
                price_per_kwh: parseInt(document.getElementById('new-tariff-price').value),
                price_per_minute: parseInt(document.getElementById('new-tariff-price-per-minute').value),
                session_fee: parseInt(document.getElementById('new-tariff-session-fee').value),
                currency: document.getElementById('new-tariff-currency').value,
            });
            hideModal();
            showToast('Тариф создан', 'success');
            await refreshTariffs();
        } catch (e) {
            showToast('Ошибка: ' + e.message, 'error');
        }
    });
}

async function deleteTariff(id) {
    if (!confirm('Удалить тариф?')) return;
    try {
        await API.tariffs.delete(id);
        showToast('Тариф удалён', 'success');
        await refreshTariffs();
    } catch (e) {
        showToast('Ошибка: ' + e.message, 'error');
    }
}
