#!/usr/bin/env python3
"""Round-2 routing audit: native ah_check toolCalls vs bash 'ah check' invocations, 6-repo corpus, 30d."""
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
AHCLI = re.compile(r'\bah\s+(check|ls|doctor|init)\b')
native_tc = collections.Counter(); bash_tc = collections.Counter(); bash_ah = collections.Counter()
native_tr = collections.Counter()
cutoff = datetime.datetime.now().timestamp() - 30*86400
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = os.path.basename(d).replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < cutoff: continue
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            if m.get('role') == 'assistant':
                for c in m.get('content', []):
                    if isinstance(c, dict) and c.get('type') == 'toolCall':
                        if c.get('name') == 'ah_check': native_tc[repo] += 1
                        elif c.get('name') == 'bash':
                            bash_tc[repo] += 1
                            cmd = str((c.get('arguments') or {}).get('command',''))
                            if AHCLI.search(cmd): bash_ah[repo] += 1
            elif m.get('role') == 'toolResult' and m.get('toolName') == 'ah_check':
                native_tr[repo] += 1
nt, bt, ba = sum(native_tc.values()), sum(bash_tc.values()), sum(bash_ah.values())
print(f'native ah_check toolCalls={nt}  native toolResults={sum(native_tr.values())}')
print(f'bash toolCalls={bt}  bash ah-CLI={ba}  ({100*ba/bt:.1f}% of bash)')
print(f'ah-check routing: native={100*nt/(nt+ba):.1f}%  bash={100*ba/(nt+ba):.1f}%  over {nt+ba} calls')
for repo in sorted(set(list(native_tc)+list(bash_ah)+list(bash_tc)+list(native_tr))):
    print(f'  {repo:28s} tc_native={native_tc[repo]:3d} tr_native={native_tr[repo]:3d} bash_ah={bash_ah[repo]:3d} bash_all={bash_tc[repo]:5d}')
