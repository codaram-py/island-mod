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
# $z0/$z1 sessions   $z2/$z3 drawer mode ('' | 'h' history | 'm' models)
# $za     session label         $zb/$zc rename draft (null = not editing)
# $zd/$ze model list state      $zj/$zk re-derive tick for $za
# $z4 tokens  $z5 refresh  $z6 open  $z7 delete  $z8 copy  $z9 count
# $zf load models  $zg pick model  $zh commit rename  $zi short model name
F_HOOKS = _js(
    'let[$z0,$z1]=(0,N.useState)(()=>window.__sessions__.list()),'
    '[$z2,$z3]=(0,N.useState)(\x01\x01),'
    '[$zb,$zc]=(0,N.useState)(null),'
    '[$zd,$ze]=(0,N.useState)(null),'
    '[$zj,$zk]=(0,N.useState)(0),'
    '$z5=()=>$z1(window.__sessions__.list()),'
    '$za=(0,N.useMemo)(()=>window.__sessions__.label(e),[e,$zj]),'
    '$z4=(0,N.useMemo)(()=>{let q=0;for(let z of e)q+=String(z.text||\x01\x01).length;'
    'return Math.ceil(q/4)},[e]),'
    '$z9=q=>q<1e3?String(q):q<1e4?(q/1e3).toFixed(1)+\x01k\x01:Math.round(q/1e3)+\x01k\x01,'
    '$zi=q=>{let z=String(q||\x01\x01).split(\x01/\x01).pop().replace(/-latest$/,\x01\x01);'
    'return z.length>13?z.slice(0,12)+\x01…\x01:z||\x01AI\x01},'
    '$z6=q=>{y.current=!0,window.__sessions__.load(q),$z3(\x01\x01),$z5(),$zk(z=>z+1)},'
    '$z7=q=>{window.__sessions__.delete(q),$z5(),$zk(z=>z+1)},'
    '$zh=()=>{$zb!==null&&(window.__sessions__.renameCurrent($zb),$zc(null),$z5(),$zk(z=>z+1))},'
    '$zf=()=>{$ze(null),n.aiModels().then(q=>$ze({l:q||[]}),q=>$ze({e:String(q)}))},'
    '$zg=q=>{let z=j.getState().settings;z&&n.settingsSet({...z,aiModel:q}),$z3(\x01\x01)},'
    '$z8=()=>{let q=e.filter(z=>!z.error&&String(z.text||\x01\x01).trim())'
    '.map(z=>\x01**\x01+(z.role===\x01user\x01?c(\x01Я\x01,\x01Me\x01):d||\x01AI\x01)+\x01**\\n\\n\x01+z.text)'
    '.join(\x01\\n\\n---\\n\\n\x01);'
    'q&&(n.clipSetText(q).catch(()=>{}),M.getState().say(c(\x01Переписка скопирована\x01,'
    '\x01Conversation copied\x01)))};'
)

F_TOPBAR = _js(
    '(0,R.jsxs)(\x01div\x01,{className:\x01flex shrink-0 items-center gap-1 px-1\x01,children:['
    '(0,R.jsx)(\x01span\x01,{className:\x01shrink-0 rounded-md bg-accent/15 px-1.5 py-0.5 '
    'text-[10px] font-semibold text-accent\x01,children:c(\x01Чат\x01,\x01Chat\x01)}),'
    # the session name, click to rename in place
    '$zb===null?(0,R.jsx)(\x01button\x01,{title:c(\x01Переименовать чат\x01,\x01Rename the chat\x01),'
    'onClick:()=>$zc($za),'
    'className:\x01press min-w-0 flex-1 truncate rounded-md px-1 py-0.5 text-left text-[12px] '
    'font-semibold hover:bg-tile ${$za?\x01text-ink-2 hover:text-ink\x01:\x01text-ink-3\x01}\x01,'
    'children:$za||c(\x01Новый чат\x01,\x01New chat\x01)}):'
    '(0,R.jsx)(\x01input\x01,{autoFocus:!0,value:$zb,onChange:q=>$zc(q.target.value),onBlur:$zh,'
    'onKeyDown:q=>{q.key===\x01Enter\x01?(q.preventDefault(),$zh()):'
    'q.key===\x01Escape\x01&&$zc(null)},'
    'onFocus:()=>j.getState().setHoldOpen(!0),'
    'className:\x01min-w-0 flex-1 rounded-md bg-tile px-1 py-0.5 text-[12px] font-semibold '
    'text-ink outline-none select-text\x01}),'
    # copy the conversation; only worth showing once there is one
    'e.length>0&&(0,R.jsx)(\x01button\x01,{title:c(\x01Скопировать переписку\x01,'
    '\x01Copy the conversation\x01),onClick:$z8,'
    'className:\x01press grid size-7 shrink-0 place-items-center rounded-lg text-ink-3 '
    'hover:bg-tile hover:text-ink\x01,children:(0,R.jsx)(k,{size:14})}),'
    '(0,R.jsx)(\x01button\x01,{title:c(\x01Новый чат\x01,\x01New chat\x01),disabled:e.length===0,'
    'onClick:()=>{y.current=!0,M.getState().reset(),$z3(\x01\x01),$zc(null),$z5(),$zk(z=>z+1)},'
    'className:\x01press grid size-7 shrink-0 place-items-center rounded-lg text-ink-3 '
    'hover:bg-tile hover:text-ink disabled:pointer-events-none disabled:opacity-30\x01,'
    'children:(0,R.jsx)(Dv,{size:15})}),'
    '(0,R.jsx)(\x01button\x01,{title:c(\x01История чатов\x01,\x01Chat history\x01),'
    'onClick:()=>{$z2===\x01h\x01?$z3(\x01\x01):($z5(),$z3(\x01h\x01))},'
    'className:\x01press grid size-7 shrink-0 place-items-center rounded-lg ${$z2===\x01h\x01?'
    '\x01bg-tile text-ink\x01:\x01text-ink-3 hover:bg-tile hover:text-ink\x01}\x01,'
    'children:(0,R.jsx)($z2===\x01h\x01?ry:rg,{size:15})})]}),'
)

F_MODELS = _js(
    '$z2===\x01m\x01?(0,R.jsx)(\x01div\x01,{className:\x01flex flex-col gap-0.5 py-1\x01,'
    'children:$zd===null?(0,R.jsxs)(\x01div\x01,'
    '{className:\x01flex items-center justify-center gap-2 py-8 text-[12px] text-ink-3\x01,'
    'children:[(0,R.jsx)(l_,{size:14,className:\x01animate-spin\x01}),'
    'c(\x01Загружаю модели…\x01,\x01Loading the models…\x01)]}):'
    '$zd.e?(0,R.jsx)(\x01div\x01,'
    '{className:\x01px-3 py-8 text-center text-[11.5px] leading-snug text-amber-300\x01,'
    'children:$zd.e}):'
    '$zd.l.length===0?(0,R.jsx)(\x01div\x01,'
    '{className:\x01px-2 py-8 text-center text-[12px] text-ink-3\x01,'
    'children:c(\x01Список моделей пуст\x01,\x01The model list is empty\x01)}):'
    '$zd.l.map(q=>(0,R.jsxs)(\x01button\x01,{onClick:()=>$zg(q),'
    'className:\x01press flex items-center gap-2 rounded-lg px-2 py-1.5 text-left text-[12.5px] '
    '${q===d?\x01bg-tile text-ink\x01:\x01text-ink-2 hover:bg-tile hover:text-ink\x01}\x01,children:['
    '(0,R.jsx)(\x01span\x01,{className:\x01min-w-0 flex-1 truncate\x01,children:q}),'
    'q===d&&(0,R.jsx)(_e,{size:13,className:\x01shrink-0 text-ok\x01})]},q))}):'
)

F_DRAWER = _js(
    'children:$z2===\x01h\x01?(0,R.jsx)(\x01div\x01,{className:\x01flex flex-col gap-0.5 py-1\x01,'
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
    'children:(0,R.jsx)(zv,{size:13})})]},q.id))}):'
)

F_BADGE = _js(
    '(0,R.jsxs)(\x01button\x01,{title:\x01${d||\x01AI\x01} · ~${$z4} ${c(\x01токенов\x01,\x01tokens\x01)} · '
    '${c(\x01клик — выбрать модель\x01,\x01click to pick a model\x01)}\x01,'
    'onClick:()=>{$z2===\x01m\x01?$z3(\x01\x01):($zf(),$z3(\x01m\x01))},'
    'className:\x01press flex h-9 w-14 shrink-0 flex-col items-center justify-center gap-px '
    'rounded-xl leading-none ${$z2===\x01m\x01?\x01bg-tile text-ink\x01:'
    '\x01text-ink-3 hover:bg-tile hover:text-ink\x01}\x01,children:['
    '(0,R.jsx)(\x01span\x01,{className:\x01w-full truncate px-0.5 text-center text-[8.5px] '
    'font-semibold\x01,children:$zi(d)}),'
    '(0,R.jsxs)(\x01span\x01,{className:\x01flex items-center gap-0.5 text-[8.5px] tabular-nums\x01,'
    'children:[(0,R.jsx)(sg,{size:9}),$z9($z4)]})]})'
)


def prefix():
    with open(os.path.join(HERE, 'session_store.js.template'), encoding='utf-8') as f:
        return f.read()


def patch_chat_tab(src):
    """Prefix the session store and rewrite Jy()'s chrome. Returns the new source."""
    src = rep(src, A_HOOKS, A_HOOKS + F_HOOKS, 'hooks')
    src = rep(src, A_SEND, A_SEND.replace('(y.current=!0,', _js('($z3(\x01\x01),y.current=!0,'), 1), 'send')
    src = rep(src, A_ROOT, A_ROOT.replace('children:[', 'children:[' + F_TOPBAR, 1), 'topbar')
    src = rep(src, A_LIST, A_LIST.replace('children:D?', F_DRAWER + F_MODELS + 'D?', 1), 'drawer')
    src = rep(src, A_NEWBTN, F_BADGE, 'badge')
    return prefix() + src
