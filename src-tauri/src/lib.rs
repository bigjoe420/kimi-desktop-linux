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
        try {
            const parsed = new URL(url, window.location.href);
            return isOAuthHost(parsed.hostname);
        } catch (e) {
            return false;
        }
    }

    function openExternal(url) {
        const invoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
        if (invoke) {
            invoke('plugin:shell|open', { path: url });
        } else {
            window.location.href = url;
        }
    }

    // --- window.open (popup-based OAuth) ---
    const originalOpen = window.open;
    window.open = function(url, target, features) {
        if (typeof url === 'string' && isOAuthUrl(url)) {
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
        // Location.href is non-configurable in this WebKit build.
    }

    // --- location.assign / location.replace ---
    const originalAssign = window.location.assign.bind(window.location);
    window.location.assign = function(url) {
        if (isOAuthUrl(url)) {
            openExternal(url);
        } else {
            originalAssign(url);
        }
    };

    const originalReplace = window.location.replace.bind(window.location);
    window.location.replace = function(url) {
        if (isOAuthUrl(url)) {
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
                e.preventDefault();
                openExternal(link.href);
            }
        }, true);

        // --- form submissions ---
        document.addEventListener('submit', function(e) {
            const form = e.target;
            if (form.action && isOAuthUrl(form.action)) {
                e.preventDefault();
                openExternal(form.action);
            }
        }, true);
    }
})();
"#;
