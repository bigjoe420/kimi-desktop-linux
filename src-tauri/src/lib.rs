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
                #[cfg(debug_assertions)]
                {
                    let _ = window.open_devtools();
                }
                let _ = window.eval(PHONE_CODE_AUTOFILL_SCRIPT);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

/// Kimi's web app requires the verification code field to be non-empty before
/// the Send button triggers SMS delivery. This helper auto-fills a dummy digit
/// when Send is clicked on an empty field.
const PHONE_CODE_AUTOFILL_SCRIPT: &str = r#"
(function() {
    if (window.__kimiPhoneHelperInstalled) return;
    window.__kimiPhoneHelperInstalled = true;

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
"#;
