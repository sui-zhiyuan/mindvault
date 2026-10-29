CubeAPI -> CubeMaster（待测试耗时 包括 e2e 和 除 CubeLet 以外部分）
    CubeAPI 鉴权/防过载限流/超时恢复/初始分布式追踪
    CubeMaster 全集群唯一管理服务，负责元数据持久化，节点选择和请求下发给 CubeLet 以及失败重试
CubeLet （待测试耗时 本身 e2e）
    节点代理，在创建流程中主要执行下面步骤
1. 生成 sandbox 元数据；解析模板/快照注解 (带测试耗时)
2. 准备 host 侧资源 包括 images, volume, storage, network, netfile, 并将元数据本地落盘  (待测试耗时)
3. 准备 host 侧 cgroup，用于限制 CubeShim 内存/cpu 使用 (待测试耗时)
4. 拉起 CubeShim 进程,创建虚拟机。 (待测试耗时)
5. 通过 containerd 调用 CubeShim create (待测试耗时)
6. 通过 containerd 调用 CubeShim wait / start (待测试耗时)
7. 调用 probe 接口，等待容器 ready (待测试耗时)
CubeShim - create （待测试耗时 本身 e2e）
    虚拟机内加载资源，初始化，虚拟机会暂停在 CubeAgent 拉起后，等待 start 调用。
1. CreatePodSandbox / LaunchVmm 创建虚拟机并启动加载资源 (待测试耗时)
2. RestoreVM 因为使用template 创建，跳过 os 加载流程，直接加载内存 (待测试耗时)
3. ResetVM 恢复暂停的时间/Timer/随机数等
CubeShim - start （待测试耗时 本身 e2e）
    直接放行 CubeAgent ，为何符合 OCI runtime 规范，方便复用 containerd 逻辑。
CubeAgent -> envd: （待测试耗时 本身 e2e）
    服务启动，监听端口，等待请求进入


CubeAPI
CubeMaster
CubeLet
CubeProxy
CubeShim
CubeAgent