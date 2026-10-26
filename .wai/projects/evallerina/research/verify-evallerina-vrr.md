# Verify evallerina-vrr: doc-findability tracer: turu (phase 4)

## Artifacts read

- Commits fab969127fe289ecd2909efee754d7422b5fc2e1 and 03a26172264520a5590b45a07f30094e94296311
- Brief: .wai/projects/evallerina/briefs/evallerina-vrr.md
- Scenario JSON: scenarios/doc-findability/turu-doc-findability-repo-key.json

## Fresh command output

```
cargo test --test evals_replay -- turu doc-findability
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/evals_replay.rs (target/debug/deps/evals_replay-ac5c597afd0dc804)

running 2 tests
test turu_doc_findability_blind_replay_is_detected ... ok
test turu_doc_findability_replays_green ... ok



test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s
```

```
cargo test --test routing -- doc_findability
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/routing.rs (target/debug/deps/routing-b40f4b992d670709)

running 2 tests
test doc_findability_row_without_turu_in_map_is_skipped_loud_never_misrouted ... ok
test doc_findability_row_routes_to_turu_whisper_never_genesis ... ok



test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 0.00s
```

```
python3 -c "import json; d=json.load(open('scenarios/doc-findability/turu-doc-findability-repo-key.json')); print(d['name'], len(d['steps']), d['recorded'])"
turu-doc-findability-repo-key 5 {'tool': 'turu', 'version': '0.11.0', 'date': '2026-10-26'}
```

```
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/evals-rotation.yml')); print('yaml ok')"
yaml ok
```

## Verdict

CONFIRM
