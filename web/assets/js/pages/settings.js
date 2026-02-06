/**
 * Settings page
 */
async function renderSettings() {
    document.getElementById('page-title').textContent = 'Настройки';

    setPage(`
        <div class="fade-in space-y-6">
            <!-- Profile -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100">
                    <h3 class="font-semibold text-gray-800">Профиль</h3>
                </div>
                <div class="p-5">
                    <form id="profile-form" class="space-y-4 max-w-md">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Email</label>
                            <input type="email" id="settings-email" disabled
                                class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-gray-50">
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Имя</label>
                            <input type="text" id="settings-name"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        </div>
                    </form>
                </div>
            </div>

            <!-- Change password -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100">
                    <h3 class="font-semibold text-gray-800">Смена пароля</h3>
                </div>
                <div class="p-5">
                    <form id="password-form" class="space-y-4 max-w-md">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Текущий пароль</label>
                            <input type="password" id="settings-old-pass" required
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Новый пароль</label>
                            <input type="password" id="settings-new-pass" required minlength="6"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Подтверждение</label>
                            <input type="password" id="settings-confirm-pass" required minlength="6"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500">
                        </div>
                        <button type="submit" class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition">
                            Сменить пароль
                        </button>
                    </form>
                </div>
            </div>

            <!-- System info -->
            <div class="bg-white rounded-xl shadow-sm border border-gray-100">
                <div class="px-5 py-4 border-b border-gray-100">
                    <h3 class="font-semibold text-gray-800">Информация о системе</h3>
                </div>
                <div class="p-5">
                    <dl class="space-y-3 text-sm max-w-md">
                        <div class="flex justify-between"><dt class="text-gray-500">Версия</dt><dd class="font-medium">Texnouz OCPP v1.6.0</dd></div>
                        <div class="flex justify-between"><dt class="text-gray-500">Протокол</dt><dd>OCPP 1.6J</dd></div>
                        <div class="flex justify-between"><dt class="text-gray-500">WebSocket</dt><dd id="settings-ws-status">—</dd></div>
                        <div class="flex justify-between"><dt class="text-gray-500">API</dt><dd><code class="text-xs bg-gray-100 px-2 py-0.5 rounded">${location.origin}/api/v1</code></dd></div>
                    </dl>
                </div>
            </div>
        </div>
    `);

    // Load profile
    try {
        const user = await API.auth.me();
        document.getElementById('settings-email').value = user.email || '';
        document.getElementById('settings-name').value = user.username || '';
    } catch (e) {
        // ignore
    }

    // WS status
    document.getElementById('settings-ws-status').innerHTML = WS.ws?.readyState === 1
        ? '<span class="text-green-600">Подключено</span>'
        : '<span class="text-gray-500">Отключено</span>';

    // Password form
    document.getElementById('password-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const oldPass = document.getElementById('settings-old-pass').value;
        const newPass = document.getElementById('settings-new-pass').value;
        const confirm = document.getElementById('settings-confirm-pass').value;

        if (newPass !== confirm) {
            showToast('Пароли не совпадают', 'error');
            return;
        }

        try {
            await API.auth.changePassword(oldPass, newPass);
            showToast('Пароль изменён', 'success');
            document.getElementById('password-form').reset();
        } catch (e) {
            showToast('Ошибка: ' + e.message, 'error');
        }
    });
}
