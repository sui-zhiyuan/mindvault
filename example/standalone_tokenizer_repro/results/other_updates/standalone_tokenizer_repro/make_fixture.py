"""Generate deterministic synthetic text. No private input or corpus is used."""
import json,random,hashlib
from pathlib import Path
from transformers import AutoTokenizer
p=Path(__file__).resolve().parent
rng=random.Random(20260921)
objects=['圆形积木','蓝色杯子','纸质笔记本','木制小船','绿色树叶','透明玻璃瓶']
verbs=['记录位置','比较颜色','计算数量','整理顺序','检查尺寸','描述形状']
lines=[]
for i in range(1600):
 a=rng.choice(objects);b=rng.choice(verbs);n=rng.randrange(1000,9999)
 lines.append(f'记录{i:04d}：请观察{a}并{b}。编号为{n}，测量值为{rng.randrange(10,100)}.{rng.randrange(100):02d}。\n')
 lines.append(f'Sample {i}: compare the shapes and describe the result in a short sentence. ')
 lines.append(json.dumps({'id':i,'count':rng.randrange(1,50),'active':bool(i%2),'label':a},ensure_ascii=False)+'\n')
text=''.join(lines)
tok=AutoTokenizer.from_pretrained(p/'tokenizer',local_files_only=True)
kwargs=dict(add_special_tokens=False,padding=False,return_attention_mask=False,return_token_type_ids=False,truncation=False)
enc=tok([text],return_offsets_mapping=True,**kwargs)
end=enc.offset_mapping[0][25599][1]
text=text[:end];ids=tok([text],**kwargs).input_ids
assert abs(len(ids[0])-25600)<=4
fixture={'texts':[text],'kwargs':kwargs,'token_ids':ids}
(p/'fixture.json').write_text(json.dumps(fixture,ensure_ascii=False))
print(json.dumps({'chars':len(text),'utf8_bytes':len(text.encode()),'tokens':len(ids[0]),'text_sha256':hashlib.sha256(text.encode()).hexdigest()}))
