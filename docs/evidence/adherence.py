#!/usr/bin/env python3
"""Measure instruction-following for incitaciones skill preambles + wai pipeline errors."""
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
req_skill = re.compile(r'Use the installed `([a-z0-9-]+)` skill')
adherence = collections.Counter()   # (skill, followed) counts
skill_reads = collections.Counter()
ah_json_exit = collections.Counter()
pipeline_errs = []
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        requested = None; seen_skill_evidence = False
        msgs = []
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            txt = ' '.join(c.get('text','') for c in m.get('content',[]) if isinstance(c,dict))
            if m.get('role') == 'user' and requested is None:
                rm = req_skill.search(txt)
                if rm: requested = rm.group(1)
                continue
            if requested is None: continue
            if m.get('role') == 'assistant':
                if requested in txt: seen_skill_evidence = True
                for c in m.get('content',[]):
                    if isinstance(c,dict):
                        if c.get('type')=='toolCall':
                            args = json.dumps(c.get('arguments',{}))
                            if requested in args and c.get('name') in ('read','bash'):
                                seen_skill_evidence = True
                            if c.get('name')=='bash' and c.get('arguments',{}).get('command','').strip().startswith('ah check'):
                                ah_json_exit['bash'] += 1
                        if c.get('type')=='text' and re.search(r'\$ ah check', txt): ah_json_exit['text-ref'] += 1
            if m.get('role') == 'toolResult' and requested in txt: seen_skill_evidence = True
            if re.search(r'wai pipeline', txt) and re.search(r'(error|Error|ERR|fail|Fail|panic|no such|not found|invalid)', txt):
                if len(pipeline_errs) < 6 and m.get('role')=='toolResult': pipeline_errs.append(txt[:200].replace('\n',' '))
        if requested:
            adherence[(requested, 'acted-on-skill') if seen_skill_evidence else (requested, 'no-evidence')] += 1
print('### incitaciones-requested skill -> evidence model acted on it')
for (sk, st), v in sorted(adherence.items()):
    print(f'  {v:3d}  {sk:28s} {st}')
print('\n### wai pipeline error samples')
for e in pipeline_errs: print('  ', e[:190])
