```
cubemastercli tpl create-from-image \
  --image 127.0.0.1:5500/cubesandbox-bench/sandbox-code-envd-ci:arm64-slim-optimal-cdb68af-20260729 \
  --writable-layer-size 1G \
  --expose-port 49983 \
  --probe 49983 \
  --probe-path /health
```


cubemastercli tpl list

cubemastercli list
cubemastercli list --all --wide  


/data/cubelet/sunrui/.bench/cube-bench -n 10 -c 10 -t bench-base --api-url "http://127.0.0.1:3000" --api-key "e2b_000000" --mode create-only


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



