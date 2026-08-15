use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval(OAUTH_INTERCEPTOR_SCRIPT);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

/// Injected into every page load. Intercepts OAuth navigations and popups so
/// they open in the system browser instead of spinning inside WebKit2GTK.
const OAUTH_INTERCEPTOR_SCRIPT: &str = r#"
(function() {
    if (window.__kimiOAuthInterceptorInstalled) return;
    window.__kimiOAuthInterceptorInstalled = true;
    console.log('[kimi-desktop-linux] OAuth interceptor installed');

    const oauthHosts = [
        'accounts.google.com',
        'appleid.apple.com',
        'login.microsoftonline.com',
        'facebook.com',
        'www.facebook.com',
        'open.weixin.qq.com',
        'graph.qq.com',
        'api.weibo.com'
    ];

    function isOAuthHost(hostname) {
        return oauthHosts.includes(hostname) || hostname.endsWith('.auth0.com');
    }

    function isOAuthUrl(url) {
        if (!url || typeof url !== 'string') return false;
        try {
            const parsed = new URL(url, window.location.href);
            return isOAuthHost(parsed.hostname);
        } catch (e) {
            return false;
        }
    }

    function showToast(message) {
        const toast = document.createElement('div');
        toast.textContent = message;
        toast.style.cssText = 'position:fixed;top:16px;left:50%;transform:translateX(-50%);background:#1a1a1a;color:#fff;padding:12px 24px;border-radius:8px;z-index:99999;font-family:sans-serif;font-size:14px;box-shadow:0 4px 12px rgba(0,0,0,0.3);';
        document.body.appendChild(toast);
        setTimeout(function() { toast.remove(); }, 4000);
    }

    function openExternal(url) {
        console.log('[kimi-desktop-linux] Opening external URL:', url);
        const invoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
        if (invoke) {
            invoke('plugin:shell|open', { path: url })
                .then(function() { console.log('[kimi-desktop-linux] Shell open succeeded'); })
                .catch(function(e) {
                    console.error('[kimi-desktop-linux] Shell open failed:', e);
                    showToast('Opening browser...');
                    setTimeout(function() { window.open(url, '_blank'); }, 100);
                });
        } else {
            console.warn('[kimi-desktop-linux] __TAURI_INTERNALS__.invoke not found, falling back');
            window.open(url, '_blank');
        }
    }

    // --- window.open (popup-based OAuth) ---
    const originalOpen = window.open;
    window.open = function(url, target, features) {
        if (typeof url === 'string' && isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted window.open:', url);
            showToast('Opening login in system browser...');
            openExternal(url);
            return null;
        }
        return originalOpen.apply(window, arguments);
    };

    // --- location.href setter (redirect-based OAuth) ---
    try {
        const hrefDescriptor = Object.getOwnPropertyDescriptor(window.location, 'href')
            || Object.getOwnPropertyDescriptor(Location.prototype, 'href');
        if (hrefDescriptor && hrefDescriptor.set) {
            Object.defineProperty(window.location, 'href', {
                set: function(url) {
                    if (isOAuthUrl(url)) {
                        console.log('[kimi-desktop-linux] Intercepted location.href:', url);
                        showToast('Opening login in system browser...');
                        openExternal(url);
                    } else {
                        hrefDescriptor.set.call(this, url);
                    }
                },
                get: hrefDescriptor.get,
                configurable: true
            });
        }
    } catch (e) {
        console.warn('[kimi-desktop-linux] Could not intercept location.href:', e);
    }

    // --- location.assign / location.replace ---
    const originalAssign = window.location.assign.bind(window.location);
    window.location.assign = function(url) {
        if (isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted location.assign:', url);
            showToast('Opening login in system browser...');
            openExternal(url);
        } else {
            originalAssign(url);
        }
    };

    const originalReplace = window.location.replace.bind(window.location);
    window.location.replace = function(url) {
        if (isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted location.replace:', url);
            showToast('Opening login in system browser...');
            openExternal(url);
        } else {
            originalReplace(url);
        }
    };

    // --- link clicks ---
    if (typeof document !== 'undefined') {
        document.addEventListener('click', function(e) {
            const link = e.target.closest('a[href]');
            if (link && isOAuthUrl(link.href)) {
                console.log('[kimi-desktop-linux] Intercepted link click:', link.href);
                e.preventDefault();
                showToast('Opening login in system browser...');
                openExternal(link.href);
            }
        }, true);

        // --- form submissions ---
        document.addEventListener('submit', function(e) {
            const form = e.target;
            if (form.action && isOAuthUrl(form.action)) {
                console.log('[kimi-desktop-linux] Intercepted form submit:', form.action);
                e.preventDefault();
                showToast('Opening login in system browser...');
                openExternal(form.action);
            }
        }, true);
    }

    console.log('[kimi-desktop-linux] OAuth interceptor ready');
})();
"#;
