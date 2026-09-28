"""Profile steady-state encoding only; setup/warmup/output validation are gated out."""
import json,sys,os
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from transformers import AutoTokenizer

ctl_path,ack_path=sys.argv[1:3]
base=Path('/root/hashbrown-sve-20260924/standalone_tokenizer_repro')
fixture=json.loads((base/'fixture.json').read_text())
tokenizer=AutoTokenizer.from_pretrained(str(base/'tokenizer'),local_files_only=True)
def encode(): return tokenizer(fixture['texts'],**fixture['kwargs']).input_ids
with ThreadPoolExecutor(max_workers=8) as pool:
    for _ in range(10): assert pool.submit(encode).result()==fixture['token_ids']
    with open(ctl_path,'w',buffering=1) as ctl,open(ack_path) as ack:
        ctl.write('enable\n');assert ack.readline().strip('\x00\r\n')=='ack'
        calls=int(os.environ.get('PROFILE_CALLS','200'))
        for _ in range(calls): result=pool.submit(encode).result()
        ctl.write('disable\n');reply=ack.readline()
        print('disable reply bytes:',repr(reply),flush=True)
        assert reply.strip('\x00\r\n')=='ack'
    assert result==fixture['token_ids']
print(f'Profiled {calls} warm encodes; output matches fixture')
