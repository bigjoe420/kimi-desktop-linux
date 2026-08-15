use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// Open a URL in the system default browser.
/// Called from the injected JS interceptor — uses Rust-side ShellExt
/// which bypasses the JS-side ACL restrictions on remote origins.
#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) {
    let _ = app.shell().open(&url, None);
}

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
        .invoke_handler(tauri::generate_handler![open_external])
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

    function showToast(message, durationMs) {
        durationMs = durationMs || 5000;
        const toast = document.createElement('div');
        toast.textContent = message;
        toast.style.cssText = 'position:fixed;top:16px;left:50%;transform:translateX(-50%);background:#1a1a1a;color:#fff;padding:12px 24px;border-radius:8px;z-index:99999;font-family:sans-serif;font-size:14px;box-shadow:0 4px 12px rgba(0,0,0,0.3);max-width:80%;word-break:break-word;';
        document.body.appendChild(toast);
        setTimeout(function() { toast.remove(); }, durationMs);
    }

    // Save original window.open BEFORE overriding — critical to break the loop
    // if the shell invoke fails and we need a fallback.
    const originalOpen = window.open;

    function openWithAnchor(url) {
        var a = document.createElement('a');
        a.href = url;
        a.target = '_blank';
        a.rel = 'noopener noreferrer';
        a.style.display = 'none';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
    }

    function openExternal(url) {
        console.log('[kimi-desktop-linux] Opening external URL:', url);
        var invoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
        if (!invoke) {
            showToast('No Tauri bridge found — trying anchor fallback');
            openWithAnchor(url);
            return;
        }

        invoke('open_external', { url: url })
            .then(function() {
                console.log('[kimi-desktop-linux] open_external succeeded');
                showToast('Opened in browser', 3000);
            })
            .catch(function(e) {
                var err = (e && e.message) ? e.message : String(e);
                console.error('[kimi-desktop-linux] open_external failed:', err);
                showToast('Error: ' + err, 8000);

                // Fallback: original window.open (new WebView window, avoids loop)
                setTimeout(function() {
                    console.log('[kimi-desktop-linux] Fallback: originalOpen');
                    originalOpen.call(window, url, '_blank');
                }, 200);
            });
    }

    // --- window.open (popup-based OAuth) ---
    window.open = function(url, target, features) {
        if (typeof url === 'string' && isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted window.open:', url);
            showToast('Opening login in system browser...', 3000);
            openExternal(url);
            return null;
        }
        return originalOpen.apply(window, arguments);
    };

    // --- location.href setter (redirect-based OAuth) ---
    try {
        var hrefDescriptor = Object.getOwnPropertyDescriptor(window.location, 'href')
            || Object.getOwnPropertyDescriptor(Location.prototype, 'href');
        if (hrefDescriptor && hrefDescriptor.set) {
            Object.defineProperty(window.location, 'href', {
                set: function(url) {
                    if (isOAuthUrl(url)) {
                        console.log('[kimi-desktop-linux] Intercepted location.href:', url);
                        showToast('Opening login in system browser...', 3000);
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
    var originalAssign = window.location.assign.bind(window.location);
    window.location.assign = function(url) {
        if (isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted location.assign:', url);
            showToast('Opening login in system browser...', 3000);
            openExternal(url);
        } else {
            originalAssign(url);
        }
    };

    var originalReplace = window.location.replace.bind(window.location);
    window.location.replace = function(url) {
        if (isOAuthUrl(url)) {
            console.log('[kimi-desktop-linux] Intercepted location.replace:', url);
            showToast('Opening login in system browser...', 3000);
            openExternal(url);
        } else {
            originalReplace(url);
        }
    };

    // --- link clicks ---
    if (typeof document !== 'undefined') {
        document.addEventListener('click', function(e) {
            var link = e.target.closest('a[href]');
            if (link && isOAuthUrl(link.href)) {
                console.log('[kimi-desktop-linux] Intercepted link click:', link.href);
                e.preventDefault();
                showToast('Opening login in system browser...', 3000);
                openExternal(link.href);
            }
        }, true);

        // --- form submissions ---
        document.addEventListener('submit', function(e) {
            var form = e.target;
            if (form.action && isOAuthUrl(form.action)) {
                console.log('[kimi-desktop-linux] Intercepted form submit:', form.action);
                e.preventDefault();
                showToast('Opening login in system browser...', 3000);
                openExternal(form.action);
            }
        }, true);
    }

    console.log('[kimi-desktop-linux] OAuth interceptor ready');

    // --- Phone login UX helper ---
    // Kimi's web app requires the verification code field to be non-empty
    // before the Send button triggers SMS delivery. This helper auto-fills
    // a dummy digit when Send is clicked on an empty field.
    (function() {
        function findCodeInput(container) {
            var inputs = container.querySelectorAll('input[type="text"], input[type="number"], input[type="tel"], input:not([type])');
            for (var i = 0; i < inputs.length; i++) {
                var input = inputs[i];
                var placeholder = (input.placeholder || '').toLowerCase();
                var name = (input.name || '').toLowerCase();
                var id = (input.id || '').toLowerCase();
                if (placeholder.indexOf('verification') !== -1 || placeholder.indexOf('code') !== -1 ||
                    name.indexOf('code') !== -1 || id.indexOf('code') !== -1) {
                    return input;
                }
            }
            return null;
        }

        document.addEventListener('click', function(e) {
            var btn = e.target.closest('button, [role="button"]');
            if (!btn) return;
            var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
            if (text !== 'send' && text !== '发送' && text !== '获取验证码') return;

            var container = btn.closest('form, div, section') || document.body;
            var codeInput = findCodeInput(container);
            if (codeInput && !codeInput.value.trim()) {
                codeInput.value = '0';
                codeInput.dispatchEvent(new Event('input', { bubbles: true }));
                codeInput.dispatchEvent(new Event('change', { bubbles: true }));
                console.log('[kimi-desktop-linux] Auto-filled dummy code for Send button');
            }
        }, true);
    })();
})();
"#;
