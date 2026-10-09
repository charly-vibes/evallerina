#!/usr/bin/env python3
"""Round-2 verify v3: 5-edit rate (both err patterns) + ah_check native text semantics, 6-repo, 30d."""
import json, glob, os, re, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
cutoff = datetime.datetime.now().timestamp() - 30*86400
edit_calls = 0; edit_errs = 0
ah_exit1 = 0; ah_exit1_ok = 0; ah_normal = 0
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < cutoff: continue
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            if m.get('role') == 'assistant':
                for c in m.get('content', []):
                    if isinstance(c, dict) and c.get('type') == 'toolCall' and c.get('name') == 'edit':
                        edit_calls += 1
            elif m.get('role') == 'toolResult':
                tn = m.get('toolName')
                t = ' '.join(x.get('text','') for x in m.get('content',[]) if isinstance(x,dict))
                if tn == 'edit':
                    if 'VALIDATION' in t.upper() or 'Could not find exact text' in t: edit_errs += 1
                elif tn == 'ah_check':
                    if 'exit 1' in t:
                        ah_exit1 += 1
                        if '"ok":true' in t or "'ok':true" in t or 'ok=true' in t:
                            ah_exit1_ok += 1
                    else: ah_normal += 1
r = edit_errs/edit_calls if edit_calls else 0
print(f'edit_calls={edit_calls} edit_errs={edit_errs} rate={r:.3f}')
print(f'ah_check native toolResults: exit-1={ah_exit1} (of which ok:true passthrough={ah_exit1_ok})  exit-0/normal={ah_normal}')
