import ctypes,hashlib,json,platform,sys,importlib.metadata,zipfile
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import tokenizers,tokenizers.tokenizers as native
w=Path('/root/hashbrown-wide-20260924');variant=sys.argv[1]
def vl(): return ctypes.CDLL(None).prctl(51,0,0,0,0)&0xffff
assert not hasattr(tokenizers,'__hashbrown_backend__'),'Application-side instrumentation leaked into clean package'
with ThreadPoolExecutor(max_workers=1) as p: worker_vl=p.submit(vl).result()
assert vl()==worker_vl==32
path=Path(native.__file__).resolve()
wheel,=list((w/'artifacts'/f'wheels-{variant}').glob('*.whl'))
extension_hash=hashlib.sha256(path.read_bytes()).hexdigest()
with zipfile.ZipFile(wheel) as z:
    member,=[n for n in z.namelist() if n.endswith('.so')]
    assert hashlib.sha256(z.read(member)).hexdigest()==extension_hash,'Installed extension differs from built wheel'
result={'variant':variant,'python':platform.python_version(),'versions':{k:importlib.metadata.version(k) for k in ['tokenizers','transformers']},'extension':str(path),'extension_sha256':extension_hash,'wheel_sha256':hashlib.sha256(wheel.read_bytes()).hexdigest(),'matches_built_wheel':True,'main_vl_bytes':vl(),'python_worker_vl_bytes':worker_vl,'rayon_vl_contract':'kernel clone inheritance; VL set before exec, application does not change VL','unmodified_python_package':True}
(w/'artifacts'/f'python-{variant}.json').write_text(json.dumps(result,indent=2))
print(json.dumps(result,indent=2))
