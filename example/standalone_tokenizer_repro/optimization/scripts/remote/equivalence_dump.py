import json
from tokenizers import Tokenizer
D='/home/wheel/hb/shared/standalone_tokenizer_repro'
tok=Tokenizer.from_file(f'{D}/tokenizer/tokenizer.json')
fx=json.load(open(f'{D}/fixture.json'))
cases={'fixture':fx['texts'][0],'empty':'','one':'a','cjk':'记录0：请观察圆形积木','combining':'he\u0301llo wo\u0308rld',
       'emoji':'🙂🚀a','trailing_cjk':'abc记录','long_ascii':'x'*5000,'newlines':'a\n\nb\r\nc'}
out={}
for k,v in cases.items():
    for ast in (False,True):
        e=tok.encode(v,add_special_tokens=ast)
        out[f'{k}|ast={ast}']={'ids':e.ids,'offsets':e.offsets,'tokens':e.tokens,'type_ids':e.type_ids,'special':e.special_tokens_mask}
for ast in (False,True):
    e=tok.encode('记录abc','🙂def',add_special_tokens=ast)
    out[f'pair|ast={ast}']={'ids':e.ids,'offsets':e.offsets,'tokens':e.tokens}
print(json.dumps(out,ensure_ascii=False,sort_keys=True))
