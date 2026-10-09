#!/usr/bin/env python3
"""Strict: did model READ the SKILL.md file or invoke /skill:X for the requested skill?"""
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
req_skill = re.compile(r'Use the installed `([a-z0-9-]+)` skill')
strict = collections.Counter()
detail = collections.Counter()
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = os.path.basename(d).replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        requested = None; how = None
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            if m.get('role') == 'user' and requested is None:
                rm = req_skill.search(' '.join(c.get('text','') for c in m.get('content',[]) if isinstance(c,dict)))
                if rm: requested = rm.group(1)
                continue
            if requested is None or how: continue
            if m.get('role') == 'assistant':
                for c in m.get('content',[]):
                    if isinstance(c,dict) and c.get('type')=='toolCall':
                        args = json.dumps(c.get('arguments',{}))
                        if re.search(rf'skills/{requested}/SKILL\.md', args) and c.get('name')=='read':
                            how = 'read-skillfile'
                        elif re.search(rf'/skill:{requested}', args) and c.get('name')=='bash':
                            how = 'invoke-via-bash'
                        elif re.search(rf'incitaciones', args): how = 'read-incitaciones'
        if requested: strict[(requested, how or 'no-evidence')] += 1
print('### strict adherence (SKILL.md read or /skill: invoked)')
for (sk, how), v in sorted(strict.items()):
    print(f'  {v:3d}  {sk:26s} {how}')
