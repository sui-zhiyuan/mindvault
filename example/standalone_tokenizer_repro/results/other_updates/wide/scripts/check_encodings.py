import json,sys,hashlib
from pathlib import Path
from transformers import AutoTokenizer
w=Path('/root/hashbrown-wide-20260924');old=Path('/root/hashbrown-sve-20260924/standalone_tokenizer_repro')
fixture=json.loads((old/'fixture.json').read_text())
texts=[fixture['texts'][0],'','hello world','a\u0301汉字🙂\n\t / café кириллица 12345']
t=AutoTokenizer.from_pretrained(str(old/'tokenizer'),local_files_only=True)
records=[]
for text in texts:
    batch=t([text],**fixture['kwargs']);e=batch.encodings[0]
    records.append({k:getattr(e,k) for k in ['ids','tokens','offsets','word_ids','sequence_ids','attention_mask','type_ids','special_tokens_mask']})
# Normalize tuple/list representations exactly as saved JSON.
records=json.loads(json.dumps(records))
oracle=w/'artifacts/encoding-oracle.json'
variant=sys.argv[1]
if variant=='stock': oracle.write_text(json.dumps(records,ensure_ascii=False))
else: assert records==json.loads(oracle.read_text()),f'Encoding mismatch: {variant}'
result={'variant':variant,'cases':len(records),'all_encoding_fields_match':True,'digest':hashlib.sha256(json.dumps(records,sort_keys=True,separators=(',',':')).encode()).hexdigest()}
(w/'artifacts'/f'encoding-check-{variant}.json').write_text(json.dumps(result,indent=2))
print(json.dumps(result))
