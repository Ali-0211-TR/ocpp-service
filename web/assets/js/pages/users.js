/**
 * Users management page (placeholder — depends on backend user endpoints)
 */
async function renderUsers() {
    document.getElementById('page-title').textContent = 'Пользователи';

    setPage(`
        <div class="fade-in space-y-4">
            <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-8 text-center">
                <div class="w-16 h-16 bg-indigo-50 rounded-full flex items-center justify-center mx-auto mb-4">
                    <svg class="w-8 h-8 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"/>
                    </svg>
                </div>
                <h3 class="text-lg font-semibold text-gray-800 mb-2">Управление пользователями</h3>
                <p class="text-gray-500 text-sm max-w-md mx-auto mb-6">
                    Управление пользователями системы. Текущий пользователь:
                </p>
                <div id="users-current" class="bg-gray-50 rounded-lg p-4 max-w-sm mx-auto text-left">
                    <div class="text-sm text-gray-400">Загрузка...</div>
                </div>
            </div>
        </div>
    `);

    try {
        const user = await API.auth.me();
        document.getElementById('users-current').innerHTML = `
            <dl class="space-y-2 text-sm">
                <div class="flex justify-between"><dt class="text-gray-500">Email</dt><dd class="font-medium">${escapeHtml(user.email)}</dd></div>
                <div class="flex justify-between"><dt class="text-gray-500">Имя</dt><dd class="font-medium">${escapeHtml(user.username || '—')}</dd></div>
                <div class="flex justify-between"><dt class="text-gray-500">Роль</dt><dd>${statusBadge('accepted', user.role || 'admin')}</dd></div>
            </dl>
        `;
    } catch (e) {
        document.getElementById('users-current').innerHTML = '<div class="text-sm text-red-500">Ошибка загрузки</div>';
    }
}
