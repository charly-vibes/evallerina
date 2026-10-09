#!/usr/bin/env python3
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
tok = collections.Counter(); sess = collections.Counter()
tool_calls = collections.Counter(); tool_errs = collections.Counter()
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = os.path.basename(d).replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        sess[repo] += 1
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            tu = m.get('tokenUsage') or j.get('tokenUsage')
            if tu: tok[repo] += tu.get('input',0)+tu.get('output',0)
            if m.get('role') == 'assistant':
                for c in m.get('content',[]):
                    if isinstance(c,dict) and c.get('type')=='toolCall':
                        tool_calls[repo] += 1
                        tool_calls[f'{repo}:{c.get("name")}'] += 1
print('### sessions & tokens (input+output sum) per repo')
for repo, s in sess.most_common(): print(f'  {repo:22s} {s:4d} sess  {tok[repo]/1e6:7.2f}M tok  {tool_calls[repo]:5d} toolcalls')
print('\n### top tools per repo')
for repo, s in sess.most_common():
    top = [(k.replace(repo+':',''),v) for k,v in tool_calls.items() if k.startswith(repo+':')]
    top.sort(key=lambda x:-x[1])
    print(f'  {repo:22s} ' + ' '.join(f'{k}:{v}' for k,v in top[:7]))
