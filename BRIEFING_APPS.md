# Briefing — Lista de apps/janelas no codabar

Branch: `chat-history` (continua do commit atual).

## Pedido
Adicionar à barra uma lista horizontal das janelas top-level visíveis — igual a
área de apps da taskbar do Windows:

- **só janelas que estão no monitor onde o codabar está** (usar
  `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL)` e comparar com
  `MonitorFromWindow(codabar_hwnd, …)`)
- ícones **quadrados 36x36**, mesma largura entre si (classe CSS `.pill.square`
  já existe)
- renderizar no container `#apps` dentro do `.zone.center`, logo depois do
  botão Start (NÃO no `.right`). O layout já está pronto no `ui/bar.js`.
- ícone de cada janela, com fallback pro ícone do .exe
- click → trazer a janela pra frente (`SetForegroundWindow`, lidando com o
  foreground-lock chamando `AllowSetForegroundWindow(ASFW_ANY)` antes)
- middle-click → fechar (`WM_CLOSE`)
- hover → tooltip com o título
- a janela ativa (`GetForegroundWindow`) fica destacada (classe `.active`
  com `background: color-mix(in srgb, var(--accent) 22%, transparent)`)
- auto-refresh a cada 1s via tokio::time::interval

## Implementação em Rust (`src-tauri/src/windows.rs` novo)

- `pub fn list_windows() -> Vec<WindowInfo>`:
  - `EnumWindows` com callback que coleta `HWND`
  - filtrar: `IsWindowVisible`, `GetWindowTextLengthW > 0`, não owned
    (`GetWindow(hwnd, GW_OWNER)` == null), não tool window
    (`!(exstyle & WS_EX_TOOLWINDOW)`), não cloaked
    (`DwmGetWindowAttribute(DWMWA_CLOAKED)` == 0)
  - título via `GetWindowTextW`
  - exe path via `GetWindowThreadProcessId` + `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`
    + `QueryFullProcessImageNameW`
  - ícone: tentar `SendMessageTimeoutW(WM_GETICON, ICON_SMALL2, …)` então
    `GetClassLongPtrW(GCLP_HICONSM)`, fallback `ExtractIconExW` do .exe
  - converter ícone pra PNG base64 via `GetIconInfo` + `GetDIBits`
  - skip o próprio HWND do codabar (comparar com `app.get_webview_window().hwnd()`)

- `#[tauri::command] fn list_windows() -> Vec<WindowInfo>`
- `#[tauri::command] fn focus_window(hwnd: isize)`:
  - `AllowSetForegroundWindow(ASFW_ANY)`
  - `if IsIconic(hwnd) { ShowWindow(hwnd, SW_RESTORE) }`
  - `SetForegroundWindow(hwnd)`
- `#[tauri::command] fn close_window(hwnd: isize)`:
  - `PostMessageW(hwnd, WM_CLOSE, 0, 0)`

Estrutura:
```rust
#[derive(Serialize, Clone)]
pub struct WindowInfo {
  hwnd: isize,
  title: String,
  icon: String,        // "data:image/png;base64,…"
  active: bool,
}
```

Cache de ícones por `(hwnd, exe_path)` com invalidação quando o path muda,
pra não re-extrair toda hora. 1s de refresh é suficiente — `GetWindowText`
+ `IsWindowVisible` são rápidos; o ícone só sai do cache se o hwnd for novo.

## Frontend (`ui/bar.js`)

- Container `#apps` **já criado** dentro do `.zone.center`, logo depois do
  botão Start — não precisa criar, só popular.
- No boot: `invoke('list_windows')` + render + `setInterval(…, 1000)`
- Cada item: `<button class="pill square" data-hwnd="…" title="<título>"><img src="<icon>" width="16" height="16"></button>`
  - click: `invoke('focus_window', { hwnd: … })`
  - middle-click (mouseup, button === 1): `invoke('close_window', …)`
- Destaque do ativo: adicionar classe `.active` no botão — a cor já tá no CSS.
- Diff ao re-render pra não piscar: comparar lista anterior pelo hwnd e só
  mudar o `.active` quando possível.

## Config

Em `config.rs`:
```rust
pub struct Widgets {
  // existentes…
  pub apps: bool,       // default true
  pub apps_max: usize,  // default 20
}
```
e expor em `settings.html` como toggle + número.

## Validação

- `cargo check` tem que passar
- `cargo clippy -- -D warnings` idealmente
- commit no branch `chat-history` com mensagem: `taskbar: window list widget`

## Não fazer agora
- Mirror do tray do shell (`Shell_TrayWnd` / `ToolbarWindow32`) — fica pra depois
- Mixer, VPN, Wi-Fi, música — continuam greyed out

Começa relendo `taskbar/src-tauri/src/main.rs` e `audio.rs` pra pegar o
padrão de command + emit do projeto. Tom seco, decide por conta, prefere
assumir uma linha a interrogatório.
