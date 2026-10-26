```
cubemastercli tpl create-from-image \
  --image 127.0.0.1:5500/cubesandbox-bench/sandbox-code-envd-ci:arm64-slim-optimal-cdb68af-20260729 \
  --writable-layer-size 1G \
  --expose-port 49983 \
  --probe 49983 \
  --probe-path /health
```

# 标准测试
cubemastercli template create-from-image \
  --image cube-sandbox-int.tencentcloudcr.com/cube-sandbox/sandbox-code:latest \
  --alias sandbox-code-latest \
  --probe 49983 --probe-path /health \
  --cpu 1000 --memory 2000 --writable-layer-size 1G \
  --no-wait


# docker sha256:79972015a4a2b48b18a2165af61606d87e7d859143165fcd33f104fbd8590bee


cubemastercli tpl list

cubemastercli list
cubemastercli list --all --wide  


/data/cubelet/sunrui/.bench/cube-bench -n 100 -c 100 -t bench-base --api-url "http://127.0.0.1:3000" --api-key "e2b_000000" --mode create-only


cubecli exec -it 2f2cd71d2795435da7e3a84dd8a24f67 bash

cubemastercli cubebox destroy $(cubemastercli list --all --quiet)

cubecli container taps



cat /usr/local/services/cubetoolbox/VERSION.txt


CUBE_BASE=/usr/local/services/cubetoolbox
$CUBE_BASE/Cubelet/bin/cubelet --version
$CUBE_BASE/Cubelet/bin/cubecli --version
$CUBE_BASE/CubeMaster/bin/cubemaster --version
$CUBE_BASE/CubeMaster/bin/cubemastercli --version
$CUBE_BASE/CubeAPI/bin/cube-api --version
$CUBE_BASE/cube-shim/bin/cube-runtime --version
$CUBE_BASE/CubeOps/bin/cubeopscli --version


cd /data/cubesandbox-startup-profiler
python3 - <<'PY'
import json
p = 'capture100.local.json'
d = json.load(open(p))
d['template_id'] = 'tpl-705f6b72114d4fe9be07d843'   # 改成你要的
json.dump(d, open(p, 'w'), ensure_ascii=False, indent=2)
PY
chmod 600 capture100.local.json


[root@node03 cubesandbox-startup-profiler]# more capture100.local.json
{
  "benchmark_binary": "/data/cubelet/sunrui/.bench/cube-bench",
  "api_url": "http://127.0.0.1:3000",
  "template_id": "tpl-705f6b72114d4fe9be07d843",
  "api_key": "e2b_000000",
  "warmup": 3,
  "log_root": "/data/log",
  "metrics_url": "http://127.0.0.1:9998/v1/metrics",
  "cubemastercli": {
    "binary": "/usr/local/bin/cubemastercli",
    "address": "127.0.0.1",
    "port": 8089,
    "exclusive_template": false
  }
}