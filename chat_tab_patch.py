"""Chat session history for the Island chat tab.

Two things happen here:
  * the session store (session_store.js.template) is prefixed onto island.js, so it
    shares the module scope that holds the chat store `M` and the tauri api `n`;
  * Jy() -- the chat tab component -- grows a top bar (new chat + history drawer)
    and its old bottom-left "new chat" button becomes a token/model badge.

Identifiers $z0..$z9 are used for the new locals: they are free in island.js, so
nothing in Jy()'s scope gets shadowed.
"""
import os

BT = chr(96)
SENT = '\x01'
HERE = os.path.dirname(os.path.abspath(__file__))

ISLAND = '/assets/island-D0bnrnc7.js'


def _js(s):
    return s.replace(SENT, BT)


def rep(s, a, b, what):
    assert s.count(a) == 1, f"{what}: anchor count={s.count(a)}"
    return s.replace(a, b, 1)


# ---- anchors in the untouched Jy() ----
A_HOOKS = 'y=(0,N.useRef)(!0);'

A_ROOT = _js(
    'return(0,R.jsxs)(\x01div\x01,{className:\x01flex h-full flex-col gap-2\x01,'
    'children:[(0,R.jsx)(\x01div\x01,{ref:_,onScroll:'
)

A_LIST = _js('className:\x01scroll-thin min-h-0 flex-1 overflow-y-auto pr-1\x01,children:D?')

A_SEND = '(y.current=!0,M.getState().send(e),n.takeFocus(),v.current?.focus())'

A_NEWBTN = _js(
    '(0,R.jsx)(\x01button\x01,{title:c(\x01Новый чат\x01,\x01New chat\x01),disabled:e.length===0,'
    'onClick:()=>{y.current=!0,M.getState().reset()},'
    'className:\x01press grid size-9 shrink-0 place-items-center rounded-xl text-ink-3 '
    'hover:bg-tile hover:text-ink disabled:pointer-events-none disabled:opacity-30\x01,'
    'children:(0,R.jsx)(Dv,{size:16})})'
)

# ---- new code ----
# $z0 sessions  $z1 setSessions  $z2 drawerOpen  $z3 setDrawerOpen  $z4 tokens
# $z5 refresh   $z6 openSession  $z7 deleteSession  $z8 copyMarkdown  $z9 formatCount
F_HOOKS = _js(
    'let[$z0,$z1]=(0,N.useState)(()=>window.__sessions__.list()),'
    '[$z2,$z3]=(0,N.useState)(!1),'
    '$z5=()=>$z1(window.__sessions__.list()),'
    '$z4=(0,N.useMemo)(()=>{let q=0;for(let z of e)q+=String(z.text||\x01\x01).length;'
    'return Math.ceil(q/4)},[e]),'
    '$z9=q=>q<1e3?String(q):q<1e4?(q/1e3).toFixed(1)+\x01k\x01:Math.round(q/1e3)+\x01k\x01,'
    '$z6=q=>{y.current=!0,window.__sessions__.load(q),$z3(!1),$z5()},'
    '$z7=q=>{window.__sessions__.delete(q),$z5()},'
    '$z8=()=>{let q=e.filter(z=>!z.error&&String(z.text||\x01\x01).trim())'
    '.map(z=>\x01**\x01+(z.role===\x01user\x01?c(\x01Я\x01,\x01Me\x01):d||\x01AI\x01)+\x01**\\n\\n\x01+z.text)'
    '.join(\x01\\n\\n---\\n\\n\x01);'
    'q&&(n.clipSetText(q).catch(()=>{}),M.getState().say(c(\x01Переписка скопирована\x01,'
    '\x01Conversation copied\x01)))};'
)

F_TOPBAR = _js(
    '(0,R.jsxs)(\x01div\x01,{className:\x01flex shrink-0 items-center gap-1 px-1\x01,children:['
    '(0,R.jsx)(\x01span\x01,{className:\x01min-w-0 flex-1 truncate text-[12px] font-semibold text-ink-2\x01,'
    'children:c(\x01Чат\x01,\x01Chat\x01)}),'
    '(0,R.jsx)(\x01button\x01,{title:c(\x01Новый чат\x01,\x01New chat\x01),disabled:e.length===0,'
    'onClick:()=>{y.current=!0,M.getState().reset(),$z3(!1),$z5()},'
    'className:\x01press grid size-7 shrink-0 place-items-center rounded-lg text-ink-3 '
    'hover:bg-tile hover:text-ink disabled:pointer-events-none disabled:opacity-30\x01,'
    'children:(0,R.jsx)(Dv,{size:15})}),'
    '(0,R.jsx)(\x01button\x01,{title:c(\x01История чатов\x01,\x01Chat history\x01),'
    'onClick:()=>{$z2?$z3(!1):($z5(),$z3(!0))},'
    'className:\x01press grid size-7 shrink-0 place-items-center rounded-lg ${$z2?'
    '\x01bg-tile text-ink\x01:\x01text-ink-3 hover:bg-tile hover:text-ink\x01}\x01,'
    'children:(0,R.jsx)($z2?ry:rg,{size:15})})]}),'
)

F_DRAWER = _js(
    'children:$z2?(0,R.jsx)(\x01div\x01,{className:\x01flex flex-col gap-0.5 py-1\x01,'
    'children:$z0.length===0?(0,R.jsx)(\x01div\x01,'
    '{className:\x01px-2 py-8 text-center text-[12px] text-ink-3\x01,'
    'children:c(\x01Сохранённых чатов пока нет\x01,\x01No saved chats yet\x01)}):'
    '$z0.map(q=>(0,R.jsxs)(\x01div\x01,'
    '{className:\x01group flex items-center gap-1 rounded-lg px-2 py-1.5 hover:bg-tile\x01,children:['
    '(0,R.jsxs)(\x01button\x01,{onClick:()=>$z6(q.id),'
    'className:\x01press flex min-w-0 flex-1 flex-col items-start gap-0.5 text-left\x01,children:['
    '(0,R.jsx)(\x01span\x01,{className:\x01w-full truncate text-[12.5px] leading-tight text-ink\x01,'
    'children:q.title||c(\x01Без названия\x01,\x01Untitled\x01)}),'
    '(0,R.jsxs)(\x01span\x01,{className:\x01text-[10.5px] text-ink-3\x01,'
    'children:[q.when,\x01 · \x01,q.n]})]}),'
    '(0,R.jsx)(\x01button\x01,{title:c(\x01Удалить\x01,\x01Delete\x01),onClick:()=>$z7(q.id),'
    'className:\x01press grid size-6 shrink-0 place-items-center rounded-md text-ink-3 opacity-0 '
    'group-hover:opacity-100 hover:bg-white/10 hover:text-danger\x01,'
    'children:(0,R.jsx)(zv,{size:13})})]},q.id))}):D?'
)

F_BADGE = _js(
    '(0,R.jsxs)(\x01button\x01,{title:\x01${d||\x01AI\x01} · ~${$z4} ${c(\x01токенов\x01,\x01tokens\x01)} · '
    '${c(\x01клик — скопировать переписку\x01,\x01click to copy the conversation\x01)}\x01,'
    'disabled:e.length===0,onClick:$z8,'
    'className:\x01press flex size-9 shrink-0 flex-col items-center justify-center gap-px '
    'rounded-xl leading-none text-ink-3 hover:bg-tile hover:text-ink '
    'disabled:pointer-events-none disabled:opacity-30\x01,children:['
    '(0,R.jsx)(sg,{size:12}),'
    '(0,R.jsx)(\x01span\x01,{className:\x01text-[8.5px] font-semibold tabular-nums\x01,'
    'children:$z9($z4)})]})'
)


def prefix():
    with open(os.path.join(HERE, 'session_store.js.template'), encoding='utf-8') as f:
        return f.read()


def patch_chat_tab(src):
    """Prefix the session store and rewrite Jy()'s chrome. Returns the new source."""
    src = rep(src, A_HOOKS, A_HOOKS + F_HOOKS, 'hooks')
    src = rep(src, A_SEND, A_SEND.replace('(y.current=!0,', '($z3(!1),y.current=!0,', 1), 'send')
    src = rep(src, A_ROOT, A_ROOT.replace('children:[', 'children:[' + F_TOPBAR, 1), 'topbar')
    src = rep(src, A_LIST, A_LIST.replace('children:D?', F_DRAWER, 1), 'drawer')
    src = rep(src, A_NEWBTN, F_BADGE, 'badge')
    return prefix() + src
