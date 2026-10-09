#!/usr/bin/env python3
import json, glob, os, re, collections, datetime
BASE = os.path.expanduser('~/.pi/agent/sessions')
RELEVANT = re.compile(r'.*charly-(genesis|wai|dont|espectacular|dulce-de-leche|evallerina).*')
models = collections.Counter()
slash = collections.Counter()
first_msgs = collections.defaultdict(list)
installed_from = collections.Counter()
for d in sorted(glob.glob(BASE + '/*')):
    if not RELEVANT.match(d): continue
    repo = os.path.basename(d).replace('--var-home-sasha-para-areas-dev-gh-charly','(root)')
    for f in glob.glob(d + '/*.jsonl') + glob.glob(d + '/**/*.jsonl', recursive=True):
        if os.path.getmtime(f) < datetime.datetime.now().timestamp() - 30*86400: continue
        user_ct = 0
        for line in open(f, errors='replace'):
            try: j = json.loads(line)
            except: continue
            t = j.get('type')
            if t == 'model_change':
                models[j.get('model', '?')] += 1
            elif t == 'message' and j.get('message',{}).get('role') == 'user':
                for c in j['message'].get('content',[]):
                    if isinstance(c,dict) and c.get('type')=='text':
                        txt = c.get('text','').strip()
                        if txt.startswith('//'): slash['//'] += 1
                        elif re.match(r'^/(skill|opt|tmp|homebrew)', txt): slash[txt.split()[0]] += 1
                        for sm in re.findall(r'/skill:\S+', txt): slash[sm] += 1
                        im = re.search(r'installed-from: (\S+)', txt)
                        if im: installed_from[im.group(1)] += 1
                        if user_ct == 0:
                            first_msgs[repo].append(txt[:100].replace('\n',' '))
                            user_ct = 1
print('### models (model_change events)'); print(models.most_common(12))
print('\n### /skill: and command invocations (user msgs)'); print(slash.most_common(20))
print('\n### installed-from skill autoloads'); print(installed_from.most_common(20))
print('\n### first user message per session (themes)')
for repo, lst in sorted(first_msgs.items(), key=lambda x: -len(x[1])):
    print(f'\n--- {repo} ({len(lst)} sessions)')
    for m in lst[:8]: print(f'  {m[:95]}')
