import brotli, json, sys, os
sys.stdout.reconfigure(encoding="utf-8")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from extract import extract

BAK = r'C:\Users\fdsmu\AppData\Local\Island\island_backup.exe'
with open(BAK, 'rb') as f:
    data = f.read()

assets = json.load(open('assets.json'))
merged = json.load(open('pt_translations_v2.json', encoding='utf-8'))

allpairs = {}
per_mod = {}
for p, i in sorted(assets.items()):
    if not p.endswith('.js'):
        continue
    src = brotli.decompress(data[i['blob_off']:i['blob_off'] + i['csz']]).decode('utf-8')
    pairs = extract(src)
    per_mod[p] = pairs
    for ru, en in pairs:
        allpairs.setdefault(en, ru)
    print(f"{p.split('/')[-1]:40s} {len(pairs):5d} call sites")

static = {k: v for k, v in allpairs.items() if '${' not in k}
missing = {k: v for k, v in static.items() if k not in merged}
print(f"\nall literals {len(allpairs)} | static {len(static)} | MISSING {len(missing)}")

json.dump(allpairs, open('all_pairs.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
json.dump(missing, open('missing_round2.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
for k in missing:
    print("  " + k)
