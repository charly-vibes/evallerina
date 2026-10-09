#!/usr/bin/env python3
"""Verify two report claims: (1) edit error rate, (2) ah check --json ok:true+exit1 count."""
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
edit_calls = 0; edit_errs = 0; ok_true_exit1 = 0; ok_true_pass = 0
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            if m.get('role') == 'assistant':
                for c in m.get('content',[]):
                    if isinstance(c,dict) and c.get('type')=='toolCall' and c.get('name')=='edit': edit_calls += 1
            if m.get('role') == 'toolResult':
                t = ' '.join(x.get('text','') for x in m.get('content',[]) if isinstance(x,dict))
                if 'Could not find exact text' in t or ('VALIDATION' in t.upper() and m.get('toolCallId')): edit_errs += 1
                try: a = json.loads(t)
                except: continue
                if isinstance(a,dict):
                    if a.get('ok') is True and 'exit 1' in t: ok_true_exit1 += 1
                    elif a.get('ok') is True: ok_true_pass += 1
print(f'edit_calls={edit_calls} edit_errs={edit_errs} rate={edit_errs/edit_calls:.3f}' if edit_calls else 'no edits')
print(f'ah/ah_check ok:true+exit1={ok_true_exit1}  ok:true-normal={ok_true_pass}')
