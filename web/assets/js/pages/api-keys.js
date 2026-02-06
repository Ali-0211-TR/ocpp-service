/**
 * API Keys management page
 */
async function renderApiKeys() {
    document.getElementById('page-title').textContent = 'API Ключи';

    setPage(`
        <div class="fade-in space-y-4">
            <div class="flex items-center justify-between">
                <p class="text-sm text-gray-500">Управление API ключами для внешних интеграций</p>
                <button onclick="showAddApiKeyModal()" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    Создать ключ
                </button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div id="apikeys-table" class="overflow-x-auto">
                    <div class="p-8 text-center text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    await refreshApiKeys();
}

let allApiKeys = [];

async function refreshApiKeys() {
    try {
        allApiKeys = await API.apiKeys.list() || [];
        renderApiKeysTable(allApiKeys);
    } catch (e) {
        showToast('Ошибка загрузки API ключей: ' + e.message, 'error');
    }
}

function renderApiKeysTable(keys) {
    const container = document.getElementById('apikeys-table');
    if (!container) return;

    if (!keys.length) {
        container.innerHTML = '<div class="p-8 text-center text-gray-400">Нет API ключей</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Название</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Ключ</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Активен</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Последнее использование</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Создан</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Действия</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${keys.map(k => `
                    <tr class="hover:bg-gray-50 transition">
                        <td class="px-5 py-3 font-medium text-gray-900">${escapeHtml(k.name || '—')}</td>
                        <td class="px-5 py-3">
                            <code class="text-xs bg-gray-100 px-2 py-0.5 rounded font-mono">${escapeHtml(k.prefix || '****')}...</code>
                        </td>
                        <td class="px-5 py-3">${k.is_active !== false
                            ? '<span class="text-green-500">✓ Активен</span>'
                            : '<span class="text-red-500">✗ Деактивирован</span>'
                        }</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${k.last_used_at ? timeAgo(k.last_used_at) : 'Никогда'}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(k.created_at)}</td>
                        <td class="px-5 py-3">
                            <button onclick="revokeApiKey('${k.id}')" class="text-red-500 hover:text-red-700 text-xs font-medium">Отозвать</button>
                        </td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
    `;
}

function showAddApiKeyModal() {
    showModal(`
        <div class="p-6">
            <h3 class="text-lg font-semibold text-gray-900 mb-4">Создать API ключ</h3>
            <form id="add-apikey-form" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Название *</label>
                    <input type="text" id="new-apikey-name" required
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="Внешняя интеграция">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Описание *</label>
                    <input type="text" id="new-apikey-description" required
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="Описание назначения ключа">
                </div>
                <div class="flex justify-end gap-3 pt-2">
                    <button type="button" onclick="hideModal()" class="px-4 py-2 text-gray-700 text-sm font-medium rounded-lg hover:bg-gray-100 transition">Отмена</button>
                    <button type="submit" class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">Создать</button>
                </div>
            </form>
        </div>
    `);

    document.getElementById('add-apikey-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        try {
            const result = await API.apiKeys.create({
                name: document.getElementById('new-apikey-name').value,
                description: document.getElementById('new-apikey-description').value,
            });

            // Show the generated key (only shown once!)
            hideModal();
            if (result.key) {
                showModal(`
                    <div class="p-6">
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">API ключ создан!</h3>
                        <p class="text-sm text-yellow-600 bg-yellow-50 p-3 rounded-lg mb-4">
                            ⚠️ Скопируйте ключ сейчас! Он не будет показан снова.
                        </p>
                        <div class="bg-gray-900 text-green-400 p-4 rounded-lg font-mono text-sm break-all">
                            ${escapeHtml(result.key)}
                        </div>
                        <div class="flex justify-end mt-4">
                            <button onclick="navigator.clipboard.writeText('${escapeHtml(result.key)}').then(() => showToast('Скопировано!', 'success')); hideModal();"
                                class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                                📋 Копировать и закрыть
                            </button>
                        </div>
                    </div>
                `);
            }
            await refreshApiKeys();
        } catch (e) {
            showToast('Ошибка: ' + e.message, 'error');
        }
    });
}

async function revokeApiKey(id) {
    if (!confirm('Отозвать этот API ключ?')) return;
    try {
        await API.apiKeys.revoke(id);
        showToast('Ключ отозван', 'success');
        await refreshApiKeys();
    } catch (e) {
        showToast('Ошибка: ' + e.message, 'error');
    }
}
