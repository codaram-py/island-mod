import brotli, json, os, sys
sys.stdout.reconfigure(encoding="utf-8")

BAK = os.environ.get('ISLAND_BAK', r'C:\Users\fdsmu\AppData\Local\Island\island_backup.exe')

BT = chr(96)
BS = chr(92)


def read_bt(s, i):
    """s[i] must be a backtick. Returns (content, index_after_closing_backtick) or None."""
    out = []
    i += 1
    while i < len(s):
        ch = s[i]
        if ch == BS:
            out.append(s[i:i + 2])
            i += 2
            continue
        if ch == BT:
            return ''.join(out), i + 1
        if ch == '$' and i + 1 < len(s) and s[i + 1] == '{':
            depth = 0
            j = i + 1
            while j < len(s):
                if s[j] == '{':
                    depth += 1
                elif s[j] == '}':
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            out.append(s[i:j])
            i = j
            continue
        out.append(ch)
        i += 1
    return None


def has_cyr(t):
    return any('\u0400' <= c <= '\u04ff' for c in t)


def extract(src):
    pairs = []
    i = 0
    while True:
        i = src.find('(' + BT, i)
        if i < 0:
            break
        r1 = read_bt(src, i + 1)
        if not r1:
            i += 1
            continue
        a1, j = r1
        if j < len(src) and src[j] == ',' and j + 1 < len(src) and src[j + 1] == BT:
            r2 = read_bt(src, j + 1)
            if r2:
                a2, k = r2
                if k < len(src) and src[k] == ')' and has_cyr(a1) and not has_cyr(a2):
                    pairs.append((a1, a2))
        i += 1
    return pairs


if __name__ == '__main__':
    with open(BAK, 'rb') as f:
        orig = f.read()

    srcs = {
        'island.js':    brotli.decompress(orig[9731348:9731348 + 239466]).decode('utf-8'),
        'settings.js':  brotli.decompress(orig[10072089:10072089 + 33008]).decode('utf-8'),
        'LangRoot.js':  brotli.decompress(orig[10001029:10001029 + 69191]).decode('utf-8'),
        'Liquid.js':    brotli.decompress(orig[9970840:9970840 + 30161]).decode('utf-8'),
        'undo2.js':     brotli.decompress(orig[10105123:10105123 + 3340]).decode('utf-8'),
    }

    allpairs = {}
    for name, s in srcs.items():
        p = extract(s)
        print(f"{name:14s}: {len(p)} call sites")
        for ru, en in p:
            allpairs.setdefault(en, ru)

    print(f"\nunique English literals: {len(allpairs)}")

    tr = json.load(open('pt_translations.json', encoding='utf-8'))
    missing = {en: ru for en, ru in allpairs.items() if en not in tr}
    print(f"already translated      : {len(allpairs) - len(missing)}")
    print(f"MISSING translations    : {len(missing)}")

    json.dump(missing, open('missing_en.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    json.dump(allpairs, open('all_pairs.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)

    for en in list(missing)[:50]:
        print("  " + en)
