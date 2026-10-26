import brotli, json, sys, os
sys.stdout.reconfigure(encoding="utf-8")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from extract import extract

BAK = r'C:\Users\fdsmu\AppData\Local\Island\island_backup.exe'
EXE = r'C:\Users\fdsmu\AppData\Local\Island\island.exe'
BT = chr(96)

with open(BAK, 'rb') as f:
    data = f.read()

assets = json.load(open('assets.json'))
merged = json.load(open('pt_translations_v2.json', encoding='utf-8'))
J = lambda d: json.dumps(d, ensure_ascii=False, separators=(',', ':'))

SRC = {}
for p, i in assets.items():
    if p.endswith('.js'):
        SRC[p] = brotli.decompress(data[i['blob_off']:i['blob_off'] + i['csz']]).decode('utf-8')

LR = '/assets/LangRoot-B1oi2SO4.js'
ST = '/assets/settings-TI-OGeDC.js'
LQ = '/assets/Liquid-IWWYSV_r.js'

# ---- own static translations per module ----
own = {}
for p, src in SRC.items():
    stat = sorted({en for ru, en in extract(src) if '${' not in en})
    own[p] = {k: merged[k] for k in stat if k in merged}

# ---- patch LangRoot source (4 language-logic patches) ----
def rep(s, a, b):
    assert s.count(a) == 1, f"count={s.count(a)}"
    return s.replace(a, b, 1)

lr = SRC[LR]
lr = rep(lr, 'function y(){return(typeof navigator<`u`?navigator.languages?.length?navigator.languages:[navigator.language]:[]).some(e=>v.test(e??``))?`ru`:`en`}',
             'function y(){var L=typeof navigator<`u`?navigator.languages?.length?navigator.languages:[navigator.language]:[];return L.some(e=>v.test(e??``))?`ru`:L.some(e=>/^pt/i.test(e??``))?`pt`:`en`}')
lr = rep(lr, 'function b(e){return e===`ru`||e===`en`?e:y()}',
             'function b(e){return e===`ru`||e===`en`||e===`pt`?e:y()}')
lr = rep(lr, 'function S(e,t){return x===`ru`?e:t}',
             'function S(e,t){if(x===`ru`)return e;if(x===`pt`){var p=window.__PT__;if(p){var q=p[t];if(q)return q}}return t}')
lr = rep(lr, 'function C(){return x===`ru`?`ru-RU`:`en-US`}',
             'function C(){return x===`ru`?`ru-RU`:x===`pt`?`pt-BR`:`en-US`}')
SRC[LR] = lr

# ---- inject OpenCode as a chat provider ----
OC = (',{id:' + BT + 'opencode' + BT
      + ',name:' + BT + 'OpenCode (local)' + BT
      + ',baseUrl:' + BT + 'http://127.0.0.1:4747/v1' + BT
      + ',model:' + BT + 'mimo-v2.6-flash' + BT
      + ',keyUrl:' + BT + 'https://console.opencode.ai' + BT
      + ',free:!0'
      + ',note:()=>i(' + BT + 'OpenCode: free модели' + BT
      + ',' + BT + 'OpenCode free models via local proxy on :4747.' + BT + ')}')
APINET_END = ('A paid gateway billed in roubles.' + BT + ')}')
assert SRC[ST].count(APINET_END) == 1, f"apinet anchor hits={SRC[ST].count(APINET_END)}"
SRC[ST] = SRC[ST].replace(APINET_END, APINET_END + OC, 1)
print(f"opencode provider injected ({len(OC)} chars)")

# ---- patch settings dropdown ----
anchor = ('value:e.language||' + BT + 'auto' + BT + ',onChange:e=>$({language:e}),options:[{value:' + BT + 'auto' + BT
          + ',label:i(' + BT + '\u041a\u0430\u043a \u0432 Windows' + BT + ',' + BT + 'Windows' + BT + ')},{value:'
          + BT + 'ru' + BT + ',label:' + BT + '\u0420\u0443\u0441\u0441\u043a\u0438\u0439' + BT + '},{value:'
          + BT + 'en' + BT + ',label:' + BT + 'English' + BT + '}]')
SRC[ST] = rep(SRC[ST], anchor, anchor[:-1] + ',{value:' + BT + 'pt' + BT + ',label:' + BT + 'Portugu\u00eas' + BT + '}]')

# ---- release notes: add a third, portuguese half ----
# they ship as "russian \n---\n english"; the reader only keeps lines starting with "- ".
import re as _re
BS = chr(92)

OLD_SEL = ('let[n,r]=e.replace(/' + BS + 'r' + BS + 'n/g,' + BT + '\n' + BT + ').split(/'
           + BS + 'n---' + BS + 'n/);return(t===' + BT + 'en' + BT + '&&r?.trim()?r:n)')
NEW_SEL = ('let q=e.replace(/' + BS + 'r' + BS + 'n/g,' + BT + '\n' + BT + ').split(/'
           + BS + 'n---' + BS + 'n/),n=q[0],r=q[1],p=q[2];return(t===' + BT + 'pt' + BT
           + '&&p?.trim()?p:t!==' + BT + 'ru' + BT + '&&r?.trim()?r:n)')
sel_hits = [p for p in SRC if OLD_SEL in SRC[p]]
assert sel_hits, "release-notes selector not found"
for p in sel_hits:
    SRC[p] = SRC[p].replace(OLD_SEL, NEW_SEL)
    print(f"release-notes selector -> {p.split('/')[-1]}")

relpt = json.load(open('relnotes_pt.json', encoding='utf-8'))
for v, bl in relpt.items():
    for b in bl:
        assert BT not in b and '${' not in b, f"unsafe char in {v}"


def note_spans(s):
    out = {}
    for m in _re.finditer(r'(\w+)=' + BT + r'(Island (1' + BS + r'.' + BS + r'd+' + BS + r'.' + BS + r'd+))', s):
        start = m.end() - len(m.group(2))
        j = start
        while j < len(s):
            if s[j] == BS:
                j += 2
                continue
            if s[j] == BT:
                break
            j += 1
        out[m.group(3)] = (start, j)
    return out


for p in list(SRC):
    spans = note_spans(SRC[p])
    if not spans:
        continue
    shown = sorted(spans, key=lambda v: [int(x) for x in v.split('.')], reverse=True)[:3]
    s = SRC[p]
    # edit back-to-front so earlier offsets stay valid
    for v in sorted(spans, key=lambda v: -spans[v][0]):
        a, b = spans[v]
        if v in shown:
            pt = relpt.get(v)
            assert pt, f"no portuguese release notes for {v}"
            s = s[:b] + '\n---\n\n' + '\n'.join('- ' + x for x in pt) + '\n' + s[b:]
        else:
            s = s[:a] + 'Island ' + v + s[b:]   # never rendered: only the 3 newest are
    SRC[p] = s
    print(f"release notes -> {p.split('/')[-1]}: pt added for {shown}, {len(spans)-len(shown)} old ones emptied")

def wrap(sub, src):
    if not sub:
        return src
    return 'window.__PT__=Object.assign(window.__PT__||{},' + J(sub) + ');\n' + src

def csz(p, sub):
    return len(brotli.compress(wrap(sub, SRC[p]).encode('utf-8'), quality=11))

# ---- settings.js: cheapest (shortest value) first, so the most strings land ----
st_items = sorted(own[ST].items(), key=lambda kv: len(kv[1]) + len(kv[0]))
cap = assets[ST]['csz']
lo, hi = 0, len(st_items)
while lo < hi:
    mid = (lo + hi + 1) // 2
    if csz(ST, dict(st_items[:mid])) <= cap:
        lo = mid
    else:
        hi = mid - 1
st_keep = dict(st_items[:lo])
spill = dict(st_items[lo:])
print(f"settings.js keeps {len(st_keep)}/{len(own[ST])}, spilling {len(spill)}")

# ---- spill into LangRoot, remainder into Liquid (own strings first, then cheapest spill) ----
lr_items = list(own[LR].items()) + sorted(spill.items(), key=lambda kv: len(kv[1]) + len(kv[0]))
cap_lr = assets[LR]['csz']
lo, hi = 0, len(lr_items)
while lo < hi:
    mid = (lo + hi + 1) // 2
    if csz(LR, dict(lr_items[:mid])) <= cap_lr:
        lo = mid
    else:
        hi = mid - 1
lr_keep = dict(lr_items[:lo])
spill2 = dict(lr_items[lo:])
print(f"LangRoot  keeps {len(lr_keep)}, spilling {len(spill2)}")

lq_items = list(own[LQ].items()) + sorted(spill2.items(), key=lambda kv: len(kv[1]) + len(kv[0]))
cap_lq = assets[LQ]['csz']
lo, hi = 0, len(lq_items)
while lo < hi:
    mid = (lo + hi + 1) // 2
    if csz(LQ, dict(lq_items[:mid])) <= cap_lq:
        lo = mid
    else:
        hi = mid - 1
lq_keep = dict(lq_items[:lo])
spill3 = dict(lq_items[lo:])
print(f"Liquid.js keeps {len(lq_keep)}, still unplaced: {len(spill3)}")
if spill3:
    for k in list(spill3)[:10]:
        print("   UNPLACED: " + k[:80])

# ---- final per-module maps ----
final = {p: dict(own[p]) for p in SRC}
final[ST] = st_keep
final[LR] = lr_keep
final[LQ] = lq_keep

# ---- shrink maps of small modules that have no spill destination ----
for p in list(SRC):
    if p in (ST, LR, LQ):
        continue
    cap = assets[p]['csz']
    items = sorted(final[p].items(), key=lambda kv: len(kv[1]) + len(kv[0]))
    lo, hi = 0, len(items)
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if csz(p, dict(items[:mid])) <= cap:
            lo = mid
        else:
            hi = mid - 1
    if lo < len(items):
        print(f"shrunk {p.split('/')[-1]}: {len(items)}->{lo}")
    final[p] = dict(items[:lo])

blobs = {}
for p in SRC:
    out = wrap(final[p], SRC[p])
    b = brotli.compress(out.encode('utf-8'), quality=11)
    cap = assets[p]['csz']
    name = p.split('/')[-1]
    print(f"{name:40s} {len(b):7d} / {cap:7d}  pad {cap - len(b):6d}  map {len(final[p]):4d}")
    assert len(b) <= cap, f"{name} OVERFLOW by {len(b) - cap}"
    blobs[p] = b
    open('fin_' + name, 'w', encoding='utf-8').write(out)

# ---- write binary ----
exe = bytearray(data)
ranges = []
for p, b in blobs.items():
    off, cap = assets[p]['blob_off'], assets[p]['csz']
    exe[off:off + cap] = b + b'\x00' * (cap - len(b))
    ranges.append((off, off + cap))

assert len(exe) == len(data)
bad = [i for i in range(len(data)) if exe[i] != data[i] and not any(a <= i < c for a, c in ranges)]
assert not bad, f"{len(bad)} bytes outside slots"
print("bytes changed outside slots: 0")

placed = set()
for m in final.values():
    placed |= set(m)
print(f"total distinct translations placed: {len(placed)}")

open(EXE, 'wb').write(bytes(exe))
print(f"WRITTEN {len(exe)} bytes")
