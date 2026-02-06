/**
 * API Client for OCPP Central System
 */
const API = {
    BASE: '/api/v1',
    token: localStorage.getItem('token'),

    /** Set auth token */
    setToken(token) {
        this.token = token;
        if (token) {
            localStorage.setItem('token', token);
        } else {
            localStorage.removeItem('token');
        }
    },

    /** Get auth headers */
    headers(extra = {}) {
        const h = { 'Content-Type': 'application/json', ...extra };
        if (this.token) h['Authorization'] = `Bearer ${this.token}`;
        return h;
    },

    /** Generic fetch wrapper */
    async request(method, path, body = null) {
        const opts = { method, headers: this.headers() };
        if (body) opts.body = JSON.stringify(body);

        const res = await fetch(`${this.BASE}${path}`, opts);

        if (res.status === 401) {
            this.setToken(null);
            window.location.hash = '#/login';
            throw new Error('Unauthorized');
        }

        const data = await res.json().catch(() => null);
        if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);

        // Unwrap ApiResponse { success, data, error }
        if (data && typeof data === 'object' && 'success' in data) {
            if (!data.success) throw new Error(data.error || 'Unknown error');
            return data.data;
        }
        // Unwrap PaginatedResponse { items, total, ... }
        if (data && typeof data === 'object' && 'items' in data) {
            return data.items;
        }
        return data;
    },

    get(path) { return this.request('GET', path); },
    post(path, body) { return this.request('POST', path, body); },
    put(path, body) { return this.request('PUT', path, body); },
    del(path) { return this.request('DELETE', path); },

    // ─── Auth ──────────────────────────────────
    auth: {
        async login(email, password) {
            const data = await API.post('/auth/login', { username: email, password });
            // After unwrap, data is the inner object with token
            if (data?.token) {
                API.setToken(data.token);
            }
            return data;
        },
        logout() {
            API.setToken(null);
            window.location.hash = '#/login';
        },
        me() { return API.get('/auth/me'); },
        changePassword(current, newPass) {
            return API.put('/auth/change-password', {
                current_password: current,
                new_password: newPass,
            });
        },
    },

    // ─── Charge Points ────────────────────────
    chargePoints: {
        list() { return API.get('/charge-points'); },
        get(id) { return API.get(`/charge-points/${id}`); },
        delete(id) { return API.del(`/charge-points/${id}`); },
        stats() { return API.get('/charge-points/stats'); },
        online() { return API.get('/charge-points/online'); },
    },

    // ─── Commands ──────────────────────────────
    commands: {
        remoteStart(cpId, idTag, connectorId) { return API.post(`/charge-points/${cpId}/remote-start`, { id_tag: idTag, connector_id: connectorId }); },
        remoteStop(cpId, transactionId) { return API.post(`/charge-points/${cpId}/remote-stop`, { transaction_id: transactionId }); },
        reset(cpId, type) { return API.post(`/charge-points/${cpId}/reset`, { type }); },
        unlock(cpId, connectorId) { return API.post(`/charge-points/${cpId}/unlock-connector`, { connector_id: connectorId }); },
        changeAvail(cpId, connectorId, type) { return API.post(`/charge-points/${cpId}/change-availability`, { connector_id: connectorId, type }); },
        triggerMsg(cpId, requestedMessage) { return API.post(`/charge-points/${cpId}/trigger-message`, { requested_message: requestedMessage }); },
        getConfig(cpId) { return API.get(`/charge-points/${cpId}/configuration`); },
    },

    // ─── IdTags ────────────────────────────────
    idTags: {
        list(params = '') { return API.get(`/id-tags${params}`); },
        get(id) { return API.get(`/id-tags/${id}`); },
        create(body) { return API.post('/id-tags', body); },
        update(id, body) { return API.put(`/id-tags/${id}`, body); },
        delete(id) { return API.del(`/id-tags/${id}`); },
        block(id) { return API.post(`/id-tags/${id}/block`); },
        unblock(id) { return API.post(`/id-tags/${id}/unblock`); },
    },

    // ─── Transactions ──────────────────────────
    transactions: {
        list(params = '') { return API.get(`/transactions${params}`); },
        get(id) { return API.get(`/transactions/${id}`); },
        forCP(cpId, params = '') { return API.get(`/charge-points/${cpId}/transactions${params}`); },
        active(cpId) { return API.get(`/charge-points/${cpId}/transactions/active`); },
        stats(cpId) { return API.get(`/charge-points/${cpId}/transactions/stats`); },
    },

    // ─── Tariffs ───────────────────────────────
    tariffs: {
        list() { return API.get('/tariffs'); },
        get(id) { return API.get(`/tariffs/${id}`); },
        getDefault() { return API.get('/tariffs/default'); },
        create(body) { return API.post('/tariffs', body); },
        update(id, body) { return API.put(`/tariffs/${id}`, body); },
        delete(id) { return API.del(`/tariffs/${id}`); },
        preview(body) { return API.post('/tariffs/preview-cost', body); },
    },

    // ─── Monitoring ────────────────────────────
    monitoring: {
        stats() { return API.get('/monitoring/stats'); },
        heartbeats() { return API.get('/monitoring/heartbeats'); },
        online() { return API.get('/monitoring/online'); },
    },

    // ─── API Keys ──────────────────────────────
    apiKeys: {
        list() { return API.get('/api-keys'); },
        create(body) { return API.post('/api-keys', body); },
        revoke(id) { return API.del(`/api-keys/${id}`); },
    },
};
