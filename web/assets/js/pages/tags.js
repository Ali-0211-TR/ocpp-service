/**
 * IdTags management page
 */
async function renderTags() {
    document.getElementById('page-title').textContent = 'IdТеги';

    setPage(`
        <div class="fade-in space-y-4">
            <div class="flex items-center justify-between">
                <div class="relative">
                    <input id="tag-search" type="text" placeholder="Поиск по тегу..."
                        class="pl-10 pr-4 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 w-64">
                    <svg class="w-4 h-4 text-gray-400 absolute left-3 top-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                </div>
                <button onclick="showAddTagModal()" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    Добавить тег
                </button>
            </div>

            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div id="tags-table" class="overflow-x-auto">
                    <div class="p-8 text-center text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    document.getElementById('tag-search').addEventListener('input', debounce(filterTags, 300));
    await refreshTags();
}

let allTags = [];

async function refreshTags() {
    try {
        allTags = await API.idTags.list() || [];
        filterTags();
    } catch (e) {
        showToast('Ошибка загрузки тегов: ' + e.message, 'error');
    }
}

function filterTags() {
    const search = (document.getElementById('tag-search')?.value || '').toLowerCase();
    let filtered = allTags;
    if (search) {
        filtered = filtered.filter(t => t.id_tag.toLowerCase().includes(search) || (t.parent_id_tag || '').toLowerCase().includes(search));
    }
    renderTagsTable(filtered);
}

function renderTagsTable(tags) {
    const container = document.getElementById('tags-table');
    if (!container) return;

    if (!tags.length) {
        container.innerHTML = '<div class="p-8 text-center text-gray-400">Нет тегов</div>';
        return;
    }

    container.innerHTML = `
        <table class="w-full text-sm">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">IdТег</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Статус</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Родительский тег</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Срок действия</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Создан</th>
                    <th class="px-5 py-3 text-left text-xs font-medium text-gray-500 uppercase">Действия</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-50">
                ${tags.map(t => `
                    <tr class="hover:bg-gray-50 transition">
                        <td class="px-5 py-3"><code class="text-sm bg-gray-100 px-2 py-0.5 rounded font-mono">${escapeHtml(t.id_tag)}</code></td>
                        <td class="px-5 py-3">${statusBadge(t.status, t.status)}</td>
                        <td class="px-5 py-3 text-gray-500">${escapeHtml(t.parent_id_tag || '—')}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${t.expiry_date ? formatDate(t.expiry_date) : 'Бессрочный'}</td>
                        <td class="px-5 py-3 text-gray-500 text-xs">${formatDate(t.created_at)}</td>
                        <td class="px-5 py-3">
                            <div class="flex items-center gap-2">
                                <button onclick="editTag('${escapeHtml(t.id_tag)}')" class="text-indigo-500 hover:text-indigo-700" title="Редактировать">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
                                </button>
                                <button onclick="deleteTag('${escapeHtml(t.id_tag)}')" class="text-red-500 hover:text-red-700" title="Удалить">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                                </button>
                            </div>
                        </td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
    `;
}

function showAddTagModal() {
    showModal(`
        <div class="p-6">
            <h3 class="text-lg font-semibold text-gray-900 mb-4">Добавить IdТег</h3>
            <form id="add-tag-form" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">IdТег *</label>
                    <input type="text" id="new-tag-id" required
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="RFID001234567890">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Статус</label>
                    <select id="new-tag-status" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        <option value="Accepted">Accepted</option>
                        <option value="Blocked">Blocked</option>
                        <option value="Expired">Expired</option>
                        <option value="Invalid">Invalid</option>
                        <option value="ConcurrentTx">ConcurrentTx</option>
                    </select>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Родительский тег</label>
                    <input type="text" id="new-tag-parent"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500"
                        placeholder="Опционально">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Срок действия</label>
                    <input type="datetime-local" id="new-tag-expiry"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                </div>
                <div class="flex justify-end gap-3 pt-2">
                    <button type="button" onclick="hideModal()" class="px-4 py-2 text-gray-700 text-sm font-medium rounded-lg hover:bg-gray-100 transition">Отмена</button>
                    <button type="submit" class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">Создать</button>
                </div>
            </form>
        </div>
    `);

    document.getElementById('add-tag-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        try {
            await API.idTags.create({
                id_tag: document.getElementById('new-tag-id').value,
                status: document.getElementById('new-tag-status').value,
                parent_id_tag: document.getElementById('new-tag-parent').value || null,
                expiry_date: document.getElementById('new-tag-expiry').value || null,
            });
            hideModal();
            showToast('Тег создан', 'success');
            await refreshTags();
        } catch (e) {
            showToast('Ошибка: ' + e.message, 'error');
        }
    });
}

async function editTag(idTag) {
    const tag = allTags.find(t => t.id_tag === idTag);
    if (!tag) return;

    showModal(`
        <div class="p-6">
            <h3 class="text-lg font-semibold text-gray-900 mb-4">Редактировать IdТег</h3>
            <form id="edit-tag-form" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">IdТег</label>
                    <input type="text" value="${escapeHtml(tag.id_tag)}" disabled
                        class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-gray-50">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Статус</label>
                    <select id="edit-tag-status" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        ${['Accepted', 'Blocked', 'Expired', 'Invalid', 'ConcurrentTx'].map(s =>
                            `<option value="${s}" ${tag.status === s ? 'selected' : ''}>${s}</option>`
                        ).join('')}
                    </select>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Родительский тег</label>
                    <input type="text" id="edit-tag-parent" value="${escapeHtml(tag.parent_id_tag || '')}"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                </div>
                <div class="flex justify-end gap-3 pt-2">
                    <button type="button" onclick="hideModal()" class="px-4 py-2 text-gray-700 text-sm font-medium rounded-lg hover:bg-gray-100 transition">Отмена</button>
                    <button type="submit" class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">Сохранить</button>
                </div>
            </form>
        </div>
    `);

    document.getElementById('edit-tag-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        try {
            await API.idTags.update(idTag, {
                status: document.getElementById('edit-tag-status').value,
                parent_id_tag: document.getElementById('edit-tag-parent').value || null,
            });
            hideModal();
            showToast('Тег обновлён', 'success');
            await refreshTags();
        } catch (e) {
            showToast('Ошибка: ' + e.message, 'error');
        }
    });
}

async function deleteTag(idTag) {
    if (!confirm(`Удалить тег ${idTag}?`)) return;
    try {
        await API.idTags.delete(idTag);
        showToast('Тег удалён', 'success');
        await refreshTags();
    } catch (e) {
        showToast('Ошибка: ' + e.message, 'error');
    }
}
