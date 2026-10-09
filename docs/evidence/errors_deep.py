#!/usr/bin/env python3
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
err_tool = collections.defaultdict(collections.Counter)   # repo -> toolName error count
cli_err = collections.defaultdict(collections.Counter)    # repo -> (cli,sub) error count
edit_err = collections.Counter()
ah_check_results = collections.Counter()                  # ah_check tool result text status
timeout = collections.Counter()
no_output = collections.Counter()
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = os.path.basename(d).replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            txt = ' '.join(c.get('text','') for c in m.get('content',[]) if isinstance(c,dict))
            tn = m.get('toolName','?')
            if m.get('role') == 'toolResult':
                if tn == 'ah_check':
                    stat = 'pass' if re.search(r'OK|ok|pass|green', txt, re.I) and not re.search(r'fail|error|ERR', txt, re.I) else 'fail/other'
                    ah_check_results[stat] += 1
                    if stat == 'fail/other': ah_check_results[txt[:60].replace('\n',' ')] += 1
                elif tn == 'edit' and (m.get('isError') or re.search(r'Could not find|No changes made|occurrences of the text', txt)):
                    edit_err[txt[:50].replace('\n',' ')] += 1
                if re.search(r'Command timed out', txt): timeout[repo] += 1
                if re.search(r'Command exited with code 1:\s*\n?\s*\(no output\)|exited with code 1.*no output', txt, re.S): no_output[repo] += 1
                if m.get('isError'):
                    err_tool[repo][tn] += 1
                    sm = re.search(r'\b(wai|dont|ah|genesis|bd|just|openspec|turu)\s+([a-z-]+)', txt)
                    if sm: cli_err[repo][f'{sm.group(1)} {sm.group(2)}'] += 1
print('### errors by toolName per repo')
for repo, c in sorted(err_tool.items(), key=lambda x: -sum(x[1].values())):
    print(f'{repo:28s} {sum(c.values()):4d} | ' + ' '.join(f'{k}:{v}' for k,v in c.most_common(8)))
print('\n### errors where a genesis CLI is named in the error text (cli + subcommand)')
for repo, c in sorted(cli_err.items(), key=lambda x: -sum(x[1].values())):
    print(f'--- {repo}')
    for k, v in c.most_common(20): print(f'   {v:3d} {k}')
print('\n### edit tool failures (top patterns)'); print(edit_err.most_common(12))
print('\n### ah_check native tool results'); print(ah_check_results.most_common(12))
print('\n### command timeouts per repo'); print(timeout.most_common())
print('\n### silent failures (code 1, no output) per repo'); print(no_output.most_common())
