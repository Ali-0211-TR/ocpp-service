/**
 * WebSocket client for real-time notifications
 */
class NotificationWS {
    constructor() {
        this.ws = null;
        this.listeners = {};
        this.reconnectInterval = 3000;
        this.reconnectTimer = null;
        this.connected = false;
    }

    /** Connect to notifications WebSocket */
    connect(filter = {}) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) return;

        const params = new URLSearchParams();
        if (filter.charge_point_id) params.set('charge_point_id', filter.charge_point_id);
        if (filter.event_types) params.set('event_types', filter.event_types);

        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const qs = params.toString();
        const url = `${proto}//${location.host}/api/v1/notifications/ws${qs ? '?' + qs : ''}`;

        console.log('[WS] Connecting to', url);
        this.ws = new WebSocket(url);

        this.ws.onopen = () => {
            console.log('[WS] Connected');
            this.connected = true;
            this.emit('_connected');
        };

        this.ws.onmessage = (e) => {
            try {
                const msg = JSON.parse(e.data);
                if (msg.type === 'connected') {
                    console.log('[WS] Welcome:', msg.message);
                    return;
                }
                // Emit by event type
                const eventType = msg.type;
                this.emit(eventType, msg);
                this.emit('*', msg); // Wildcard
            } catch (err) {
                console.error('[WS] Parse error:', err);
            }
        };

        this.ws.onclose = (e) => {
            console.log('[WS] Disconnected', e.code, e.reason);
            this.connected = false;
            this.emit('_disconnected');
            this.scheduleReconnect();
        };

        this.ws.onerror = (e) => {
            console.error('[WS] Error', e);
        };
    }

    /** Disconnect */
    disconnect() {
        clearTimeout(this.reconnectTimer);
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.connected = false;
    }

    /** Schedule reconnection */
    scheduleReconnect() {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = setTimeout(() => {
            if (!this.connected && API.token) {
                console.log('[WS] Reconnecting...');
                this.connect();
            }
        }, this.reconnectInterval);
    }

    /** Listen to event type */
    on(eventType, callback) {
        if (!this.listeners[eventType]) this.listeners[eventType] = [];
        this.listeners[eventType].push(callback);
        return () => this.off(eventType, callback);
    }

    /** Remove listener */
    off(eventType, callback) {
        if (this.listeners[eventType]) {
            this.listeners[eventType] = this.listeners[eventType].filter(cb => cb !== callback);
        }
    }

    /** Emit event to listeners */
    emit(eventType, data) {
        (this.listeners[eventType] || []).forEach(cb => {
            try { cb(data); } catch (e) { console.error('[WS] Listener error:', e); }
        });
    }
}

// Global instance
const WS = new NotificationWS();
