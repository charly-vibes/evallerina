#!/usr/bin/env python3
"""Analyze pi sessions last 30 days: genesis-family CLI usage, errors, skills, models."""
import json, glob, os, re, collections, datetime

BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')

tools_by_repo = collections.defaultdict(collections.Counter)
bash_first = collections.defaultdict(collections.Counter)   # repo -> first token of bash lines
wai_sub = collections.Counter()                              # wai <subcommand>
dont_sub = collections.Counter()
ah_sub = collections.Counter()
genesis_sub = collections.Counter()
just_recipes = collections.Counter()
skills = collections.Counter()                               # /skill or installed skill autoloads
models = collections.Counter()
errors_total = 0
errors_by_repo = collections.Counter()
error_samples = collections.defaultdict(collections.Counter)
sessions_by_repo = collections.Counter()
repo_sessions = collections.defaultdict(set)

for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = re.sub(r'^.*charly-?|--?$', lambda m: m.group(0), d)
    # better: extract last path segment
    repo = os.path.basename(d).replace('--','')
    repo = repo.replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        sid = os.path.basename(f)
        if sid not in repo_sessions[repo]:
            repo_sessions[repo].add(sid); sessions_by_repo[repo] += 1
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            if j.get('type') != 'message': continue
            m = j.get('message', {})
            if m.get('role') == 'toolResult':
                if m.get('isError'):
                    errors_total += 1
                    errors_by_repo[repo] += 1
                    txt = ' '.join(c.get('text','') for c in m.get('content',[]) if isinstance(c,dict))
                    key = None
                    for pat, name in [
                        (r'No such file or directory','path-missing'),
                        (r'usage:|Usage:','cli-usage-error'),
                        (r'command not found|not recognized','command-not-found'),
                        (r'Permission denied','permission'),
                        (r'not a valid value|invalid|Invalid','invalid-arg'),
                        (r'genesis','genesis-cli-fail'),
                        (r'wai','wai-cli-fail'),
                        (r'ah ','ah-cli-fail'),
                        (r'dont','dont-cli-fail'),
                    ]:
                        if re.search(pat, txt): key = name; break
                    error_samples[repo][key or txt[:40]] += 1
            if m.get('role') != 'assistant': continue
            for c in m.get('content',[]):
                if not isinstance(c,dict): continue
                if c.get('type') == 'toolCall':
                    tools_by_repo[repo][c.get('name')] += 1
                    if c.get('name') == 'bash':
                        cmd = c.get('arguments',{}).get('command','')
                        for linec in cmd.split('\n'):
                            linec = linec.strip()
                            if not linec or linec.startswith('#'): continue
                            tok = linec.split()[0]
                            if tok in ('sudo','time','env','NO_COLOR=1','cd','(cd','export','open'): 
                                if tok in ('(cd','cd'): pass
                            if tok in ('cd','time','env','export','open','sudo','(cd'):
                                continue
                            bash_first[repo][tok] += 1
                            if tok == 'wai': wai_sub[linec.split()[1] if len(linec.split())>1 else '(bare)'] += 1
                            elif tok == 'dont': dont_sub[linec.split()[1] if len(linec.split())>1 else '(bare)'] += 1
                            elif tok == 'ah': ah_sub[linec.split()[1] if len(linec.split())>1 else '(bare)'] += 1
                            elif tok == 'genesis': genesis_sub[linec.split()[1] if len(linec.split())>1 else '(bare)'] += 1
                            elif tok == 'just': just_recipes[linec.split()[1] if len(linec.split())>1 else '(bare)'] += 1
                elif c.get('type') == 'text':
                    for sm in re.findall(r'^/\S+', c.get('text',''), re.M):
                        skills[sm.split(':')[0]] += 1

print('### toolCall counts by repo')
for repo, c in sorted(tools_by_repo.items(), key=lambda x: -sum(x[1].values())):
    print(f'{sessions_by_repo[repo]:3d} sess | ' + ' '.join(f'{k}:{v}' for k,v in c.most_common()) + f'  ← {repo}')

print('\n### bash first-token by repo (genesis-family only, top 8 each)')
for repo, c in sorted(bash_first.items(), key=lambda x: -sum(x[1].values())):
    print(f'{repo:24s} ' + ' '.join(f'{k}:{v}' for k,v in c.most_common(8)))

print('\n### wai subcommands'); print(wai_sub.most_common(25))
print('\n### dont subcommands'); print(dont_sub.most_common(25))
print('\n### ah subcommands'); print(ah_sub.most_common(25))
print('\n### genesis subcommands'); print(genesis_sub.most_common(25))
print('\n### just recipes'); print(just_recipes.most_common(15))
print('\n### skill/command invocations'); print(skills.most_common(30))
print('\n### models'); print(models.most_common(10))
print(f'\n### toolResult errors: {errors_total} total')
for repo, c in sorted(errors_by_repo.items(), key=lambda x: -x[1]):
    print(f'{repo:24s} {c:4d} | ' + '; '.join(f'{k}:{v}' for k,v in error_samples[repo].most_common(6)))
