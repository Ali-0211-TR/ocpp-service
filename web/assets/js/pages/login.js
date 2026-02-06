/**
 * Login page handler
 */
function initLoginPage() {
    const form = document.getElementById('login-form');
    if (!form) return;

    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        const email = document.getElementById('login-email').value;
        const password = document.getElementById('login-password').value;
        const btn = document.getElementById('login-btn');
        const errDiv = document.getElementById('login-error');

        btn.disabled = true;
        btn.textContent = 'Вход...';
        errDiv.classList.add('hidden');

        try {
            const data = await API.auth.login(email, password);
            API.token = data.token;
            localStorage.setItem('token', data.token);
            showMainLayout();
            location.hash = '#/dashboard';
        } catch (err) {
            errDiv.textContent = err.message || 'Ошибка входа';
            errDiv.classList.remove('hidden');
        } finally {
            btn.disabled = false;
            btn.textContent = 'Войти';
        }
    });
}

/** Show login page, hide main layout */
function showLoginPage() {
    document.getElementById('login-page').classList.remove('hidden');
    document.getElementById('main-layout').classList.add('hidden');
    WS.disconnect();
}

/** Show main layout, hide login page */
function showMainLayout() {
    document.getElementById('login-page').classList.add('hidden');
    document.getElementById('main-layout').classList.remove('hidden');
    document.getElementById('main-layout').classList.add('flex');

    // Load user info
    loadUserInfo();

    // Connect WebSocket
    WS.connect();
}

/** Load current user info into sidebar */
async function loadUserInfo() {
    try {
        const user = await API.auth.me();
        document.getElementById('user-name').textContent = user.username || user.email;
        document.getElementById('user-email').textContent = user.email;
        document.getElementById('user-avatar').textContent = (user.username || user.email)[0].toUpperCase();
    } catch (e) {
        console.error('Failed to load user info:', e);
    }
}

/** Logout */
function doLogout() {
    API.auth.logout();
    showLoginPage();
    location.hash = '#/login';
}

/** Toggle sidebar on mobile */
function toggleSidebar() {
    const sidebar = document.getElementById('sidebar');
    sidebar.classList.toggle('-translate-x-full');
}

/** Show/hide modal */
function showModal(html) {
    document.getElementById('modal-content').innerHTML = html;
    document.getElementById('modal-overlay').classList.remove('hidden');
}

function hideModal() {
    document.getElementById('modal-overlay').classList.add('hidden');
    document.getElementById('modal-content').innerHTML = '';
}

// Close modal on overlay click
document.getElementById('modal-overlay')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('modal-overlay')) hideModal();
});
