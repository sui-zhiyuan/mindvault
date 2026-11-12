## A. C（86f6d9b + auto-prewrite），prewrite 开

### A. C（86f6d9b + auto-prewrite），prewrite 开：逐轮客户端与服务端总览

| 轮次 | 整轮 ms | 客户端单实例均值 ms | 客户端 P50 ms | 客户端 P95 ms | 成功/请求 |
|---|---:|---:|---:|---:|---|
| blk1-R1 | 133.401 | 101.333 | 101.069 | 129.912 | 100/100 |
| blk1-R2 | 146.514 | 112.395 | 123.761 | 143.119 | 100/100 |
| blk1-R3 | 129.644 | 94.973 | 95.389 | 128.745 | 100/100 |
| blk1-R4 | 139.466 | 102.685 | 111.968 | 135.603 | 100/100 |
| blk1-R5 | 139.119 | 106.382 | 111.771 | 137.083 | 100/100 |

整轮中位 **139.119 ms**（范围 129.6–146.5，n=5）。

#### 1. 按执行顺序排列的相邻 host 区间（每格 均值 / P95 [有效实例/总实例]，ms）

| 阶段 | R1 | R2 | R3 | R4 | R5 |
|---|---:|---:|---:|---:|---:|
| Master 开始 → Cubelet 收到（分发及等待）<br>`master_start_to_cubelet_start` | 7.4 / 14.4 [103/103] | 11.7 / 25.5 [103/103] | 6.1 / 13.2 [103/103] | 5.0 / 11.7 [103/103] | 9.5 / 19.8 [103/103] |
| Cubelet 收到 → 解析出快照路径（clone / cgroup 等前期准备）<br>`cubelet_start_to_resolved` | 15.2 / 32.3 [103/103] | 13.1 / 29.4 [103/103] | 12.2 / 23.9 [103/103] | 14.5 / 41.9 [103/103] | 12.0 / 29.8 [103/103] |
| 快照路径已解析 → Shim 收到 Create（containerd 元数据、shim start、ttrpc）<br>`resolved_to_shim_start` | 17.2 / 29.1 [103/103] | 24.5 / 46.2 [103/103] | 16.5 / 23.9 [103/103] | 20.9 / 40.8 [103/103] | 20.5 / 34.3 [103/103] |
| Shim 收到 Create → 快照初始化标记<br>`shim_start_to_snapshot_init` | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] |
| 快照初始化 → 尝试加入 cgroup 后的标记<br>`snapshot_init_to_cgroup_join` | 0.3 / 0.8 [103/103] | 1.3 / 4.6 [103/103] | 0.6 / 0.9 [103/103] | 0.3 / 1.0 [103/103] | 0.6 / 0.5 [103/103] |
| cgroup 后侧标记 → start_vm 入口<br>`cgroup_join_to_vm_start` | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] |
| start_vm → agent ready（Launch、Restore 含 prewrite、ResetVm）<br>`vm_start_to_agent_ready` | 20.2 / 22.8 [103/103] | 20.1 / 23.0 [103/103] | 21.6 / 29.5 [103/103] | 20.7 / 24.0 [103/103] | 20.6 / 23.5 [103/103] |
| agent ready → Shim Create 返回（guest sandbox/container）<br>`agent_ready_to_shim_finish` | 4.1 / 5.1 [103/103] | 3.8 / 4.7 [103/103] | 4.0 / 4.9 [103/103] | 4.2 / 5.0 [103/103] | 3.9 / 4.7 [103/103] |
| Shim 返回 → Master 返回（Start、探针、状态及后处理）<br>`shim_finish_to_master_reply` | 13.2 / 19.4 [103/103] | 12.6 / 18.3 [103/103] | 13.8 / 20.0 [103/103] | 13.0 / 18.7 [103/103] | 14.4 / 21.5 [103/103] |
| Master 返回 → API created 事件<br>`master_reply_to_api_created` | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] |

#### 2. 外层区间（包住上表若干行，不能相加）

| 阶段 | R1 | R2 | R3 | R4 | R5 |
|---|---:|---:|---:|---:|---:|
| Cubelet 收到 → Shim 收到 Create<br>`cubelet_start_to_shim_start` | 32.4 / 53.3 [103/103] | 37.6 / 59.8 [103/103] | 28.7 / 47.6 [103/103] | 35.3 / 56.1 [103/103] | 32.5 / 51.1 [103/103] |
| Shim 收到 Create → start_vm 入口<br>`shim_start_to_vm_start` | 0.6 / 1.2 [103/103] | 1.6 / 5.0 [103/103] | 1.0 / 1.2 [103/103] | 0.7 / 1.4 [103/103] | 1.0 / 0.9 [103/103] |
| Shim Create 整体<br>`shim_start_to_shim_finish` | 24.9 / 28.8 [103/103] | 25.5 / 30.6 [103/103] | 26.6 / 34.8 [103/103] | 25.6 / 29.5 [103/103] | 25.5 / 28.4 [103/103] |
| Master 开始 → Master 返回<br>`master_start_to_master_reply` | 77.9 / 103.9 [103/103] | 87.5 / 117.5 [103/103] | 75.2 / 104.9 [103/103] | 78.9 / 109.5 [103/103] | 81.8 / 107.2 [103/103] |

#### 3. 组件自报 stat（Cubelet <5 ms 不输出，样本为条件样本）

| 阶段 | R1 | R2 | R3 | R4 | R5 |
|---|---:|---:|---:|---:|---:|
| cubelet.cubebox-service<br>`cubelet.cubebox-service` | 67.0 / 88.6 [103/103] | 72.7 / 95.7 [103/103] | 65.1 / 82.4 [103/103] | 70.2 / 91.9 [103/103] | — / — [0] |
| cubelet.storage<br>`cubelet.storage` | 6.5 / 9.2 [28/103] | 7.6 / 14.1 [35/103] | 8.4 / 11.6 [28/103] | 7.5 / 9.6 [40/103] | — / — [0] |
| cubelet.network<br>`cubelet.network` | 5.8 / 6.3 [3/103] | 7.1 / 16.0 [16/103] | — / — [0] | — / — [0] | — / — [0] |
| cubelet.cgroup<br>`cubelet.cgroup` | 16.1 / 21.6 [45/103] | 7.0 / 10.9 [39/103] | 10.3 / 14.9 [55/103] | 18.6 / 27.2 [33/103] | — / — [0] |
| cubelet.create-sandbox-metadata<br>`cubelet.create-sandbox-metadata` | 9.9 / 20.4 [46/103] | 15.9 / 31.0 [64/103] | 7.1 / 11.1 [38/103] | 12.3 / 21.0 [49/103] | — / — [0] |
| cubelet.cube-sandbox-store<br>`cubelet.cube-sandbox-store` | 9.5 / 11.6 [18/103] | 11.9 / 15.7 [26/103] | 5.6 / 6.6 [16/103] | 9.8 / 16.7 [31/103] | — / — [0] |
| cubelet.sandbox-create<br>`cubelet.sandbox-create` | 36.3 / 41.9 [103/103] | 39.6 / 49.2 [103/103] | 39.1 / 46.9 [103/103] | 39.2 / 49.0 [103/103] | — / — [0] |
| cubelet.sandbox-start<br>`cubelet.sandbox-start` | 37.6 / 42.9 [103/103] | 40.8 / 49.8 [103/103] | 40.5 / 47.9 [103/103] | 40.3 / 50.0 [103/103] | — / — [0] |
| cubelet.sandbox-probe<br>`cubelet.sandbox-probe` | 7.4 / 10.2 [101/103] | 7.1 / 8.9 [103/103] | 7.4 / 8.9 [100/103] | 7.4 / 8.8 [100/103] | — / — [0] |
| cubelet.cubebox<br>`cubelet.cubebox` | 52.0 / 62.8 [103/103] | 60.1 / 80.2 [103/103] | 53.2 / 65.3 [103/103] | 56.0 / 76.3 [103/103] | — / — [0] |
| shim.LaunchVmm<br>`shim.LaunchVmm` | 1.1 / 2.0 [103/103] | 1.3 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.1 / 2.0 [103/103] | 1.1 / 2.0 [103/103] |
| shim.RestoreVm<br>`shim.RestoreVm` | 12.4 / 14.0 [103/103] | 12.4 / 16.0 [103/103] | 13.5 / 20.0 [103/103] | 12.7 / 15.0 [103/103] | 12.6 / 16.0 [103/103] |
| shim.ResetVm<br>`shim.ResetVm` | 5.0 / 7.0 [103/103] | 4.7 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.2 / 7.0 [103/103] | 5.3 / 7.0 [103/103] |
| shim.CreateSandbox<br>`shim.CreateSandbox` | 0.1 / 1.0 [103/103] | 0.0 / 0.0 [103/103] | 0.2 / 1.0 [103/103] | 0.2 / 1.0 [103/103] | 0.1 / 1.0 [103/103] |
| shim.CreateContainer<br>`shim.CreateContainer` | 2.8 / 4.0 [103/103] | 2.5 / 3.0 [103/103] | 2.6 / 3.0 [103/103] | 2.8 / 4.0 [103/103] | 2.6 / 3.0 [103/103] |
| shim.CreatePodSandbox<br>`shim.CreatePodSandbox` | 24.4 / 28.0 [103/103] | 25.0 / 30.0 [103/103] | 26.0 / 34.0 [103/103] | 25.1 / 29.0 [103/103] | 25.0 / 28.0 [103/103] |

#### 4. 最后完成者的关键路径（去掉 3 个串行预热；末 10% 对比中间 50%，均值 ms）

```text
CAP-c100-r1: n=100 (warm-ups dropped) last master_reply at +130.1 ms, arrivals spread 35.8 ms
CAP-c100-r2: n=100 (warm-ups dropped) last master_reply at +144.2 ms, arrivals spread 37.9 ms
CAP-c100-r3: n=100 (warm-ups dropped) last master_reply at +126.6 ms, arrivals spread 33.2 ms
CAP-c100-r4: n=100 (warm-ups dropped) last master_reply at +136.3 ms, arrivals spread 35.1 ms
CAP-c100-r5: n=100 (warm-ups dropped) last master_reply at +135.5 ms, arrivals spread 38.8 ms

segment (ms, mean)        mid 50%   slowest 10%   delta
arrival                        23.7         26.6     +2.9
master->cubelet                 8.4         13.7     +5.3
cubelet->resolved              11.6         29.3    +17.7
resolved->shim                 24.4         21.8     -2.6
shim->vm_start                  0.7          1.3     +0.6
vm_start->agent                21.0         21.5     +0.5
agent->shim_done                4.0          4.6     +0.6
shim_done->reply               12.2         14.9     +2.7
completion                    106.1        133.7    +27.6
shim.RestoreVm_ms              13.2         12.7     -0.6  (logged n=250/50)
shim.ResetVm_ms                 5.0          5.9     +0.9  (logged n=250/50)
cubelet.sandbox-probe_ms        7.2          7.3     +0.2  (logged n=200/40)
cubelet.cgroup_ms               9.7         16.1     +6.5  (logged n=66/39)
cubelet.storage_ms              7.0          8.0     +1.0  (logged n=53/33)
cubelet.network_ms              7.2          5.9     -1.3  (logged n=6/7)
cubelet.create-sandbox-metadata_ms     13.2          8.7     -4.6  (logged n=133/24)
```

#### 5. OS 级阻塞（bpftrace，单独诊断轮，非计时样本）

```text
== syscalls by total time (top 15 per process kind)
cubelet  futex              total=  84220.0 ms n=   8680 avg=  9702.8 us  >1ms=788
cubelet  epoll_pwait        total=   5702.1 ms n=   7577 avg=   752.6 us  >1ms=189
cubelet  nanosleep          total=    678.1 ms n=   4204 avg=   161.3 us  >1ms=63
cubelet  ioctl              total=    666.9 ms n=    109 avg=  6118.3 us  >1ms=69
cubelet  close              total=    552.9 ms n=   8255 avg=    67.0 us  >1ms=73
cubelet  openat             total=    398.7 ms n=   7275 avg=    54.8 us  >1ms=26
cubelet  waitid             total=    383.1 ms n=    107 avg=  3580.5 us  >1ms=104
cubelet  read               total=    172.9 ms n=  10525 avg=    16.4 us  >1ms=54
cubelet  clone              total=    119.6 ms n=    103 avg=  1161.6 us  >1ms=43
cubelet  pipe2              total=    113.1 ms n=    309 avg=   366.0 us  >1ms=47
cubelet  recvfrom           total=     95.4 ms n=     54 avg=  1766.4 us  >1ms=48
cubelet  epoll_ctl          total=     84.9 ms n=  11093 avg=     7.7 us  >1ms=0
cubelet  newfstatat         total=     72.1 ms n=  12839 avg=     5.6 us  >1ms=0
cubelet  write              total=     38.4 ms n=   3354 avg=    11.4 us  >1ms=3
cubelet  accept4            total=     36.5 ms n=    318 avg=   114.8 us  >1ms=9
shim     epoll_pwait        total= 781356.3 ms n=  22296 avg= 35044.7 us  >1ms=3251
shim     futex              total= 452533.2 ms n=  49773 avg=  9091.9 us  >1ms=3844
shim     ioctl              total=    732.5 ms n= 159032 avg=     4.6 us  >1ms=103
shim     madvise            total=    300.9 ms n= 124343 avg=     2.4 us  >1ms=0
shim     write              total=    198.9 ms n=  37121 avg=     5.4 us  >1ms=15
shim     clone3             total=    189.5 ms n=   3192 avg=    59.4 us  >1ms=0
shim     clock_nanosleep    total=    153.3 ms n=    145 avg=  1057.4 us  >1ms=145
shim     prctl              total=    139.4 ms n=   4126 avg=    33.8 us  >1ms=97
shim     read               total=    127.5 ms n=  12978 avg=     9.8 us  >1ms=44
shim     mprotect           total=    101.1 ms n=  29845 avg=     3.4 us  >1ms=0
shim     mmap               total=     85.7 ms n=  15062 avg=     5.7 us  >1ms=0
shim     nanosleep          total=     61.0 ms n=    573 avg=   106.5 us  >1ms=2
shim     openat             total=     39.8 ms n=   9251 avg=     4.3 us  >1ms=0
shim     munmap             total=     27.8 ms n=   5768 avg=     4.8 us  >1ms=0
shim     recvfrom           total=     15.4 ms n=   9693 avg=     1.6 us  >1ms=0

== D-state (uninterruptible) waits by kernel stack (top 12)
cubelet  total=   486.3 ms n=    70 avg=    6946 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+172 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
cubelet  total=   145.6 ms n=    19 avg=    7663 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+240 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
shim     total=    24.8 ms n=     1 avg=   24761 us | wait_for_common+204 < wait_for_completion+32 < __wait_rcu_gp+348 < synchronize_rcu+144 < rcu_sync_enter+284 < percpu_down_write+52 < cgroup_procs_write_start+128
shim     total=    22.5 ms n=  1376 avg=      16 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_set_irq_routing+552 < kvm_vm_ioctl+1064 < __arm64_sys_ioctl+180
cubelet  total=     8.8 ms n=    70 avg=     125 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone+124 < __arm64_sys_clone+40 < invoke_syscall+80
shim     total=     8.2 ms n=   609 avg=      13 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_io_bus_register_dev+624 < kvm_assign_ioeventfd_idx+464 < kvm_ioeventfd+144
shim     total=     8.0 ms n=   609 avg=      13 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_io_bus_register_dev+624 < kvm_assign_ioeventfd_idx+464 < kvm_ioeventfd+456
shim     total=     6.8 ms n=    64 avg=     107 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone3+200 < __arm64_sys_clone3+28 < invoke_syscall+80
shim     total=     4.9 ms n=   146 avg=      34 us | rwsem_down_write_slowpath+512 < down_write_killable+100 < vm_mmap_pgoff+148 < ksys_mmap_pgoff+476 < __arm64_sys_mmap+56 < invoke_syscall+80 < el0_svc_common.constprop.0+200
shim     total=     4.7 ms n=   211 avg=      22 us | rwsem_down_write_slowpath+512 < down_write_killable+100 < do_mprotect_pkey.constprop.0+172 < __arm64_sys_mprotect+36 < invoke_syscall+80 < el0_svc_common.constprop.0+200 < do_el0_svc+72
cubelet  total=     4.6 ms n=    36 avg=     127 us | xlog_wait_on_iclog+368 < xlog_force_lsn+476 < xfs_log_force_seq+204 < xfs_log_force_inode+120 < xfs_dir_fsync+48 < vfs_fsync_range+60 < __arm64_sys_fsync+72
cubelet  total=     4.5 ms n=    38 avg=     117 us | wait_for_common+204 < wait_for_completion+32 < __flush_workqueue+292 < xlog_cil_push_now.isra.0+56 < xlog_cil_force_seq+136 < xfs_log_force_seq+164 < xfs_log_force_inode+120
```

#### 6. 二级探针（cgroup rstat flush、reflink 源文件、Cubelet 内核态 on-CPU）

#### 7. Cubelet Go trace

Cubelet 系统调用阻塞（Go trace，3 s 窗口，按函数自身）：
```text
     1.03s 35.30% 35.30%      1.03s 35.30%  syscall.Close
     0.72s 24.70% 60.01%      0.72s 24.70%  syscall.openat
     0.37s 12.63% 72.64%      0.37s 12.63%  github.com/tencentcloud/CubeSandbox/Cubelet/pkg/cubecow._Cfunc_cubecow_create_snapshot_from_volume_go
     0.31s 10.57% 83.21%      0.31s 10.57%  internal/syscall/unix.Waitid
     0.12s  4.03% 87.23%      0.12s  4.03%  syscall.readlen
     0.08s  2.83% 90.06%      0.08s  2.83%  syscall.read
     0.08s  2.59% 92.65%      0.08s  2.59%  syscall.fstatat
     0.06s  2.21% 94.86%      0.06s  2.21%  syscall.accept4
     0.04s  1.44% 96.30%      0.04s  1.44%  syscall.write
     0.03s  0.92% 97.23%      0.03s  0.92%  golang.org/x/sys/unix.fcntl
```

## B. BEST（v0.7.1 + #36 + F1 + #37 + F2 + auto-prewrite），prewrite 开

### B. BEST（v0.7.1 + #36 + F1 + #37 + F2 + auto-prewrite），prewrite 开：逐轮客户端与服务端总览

| 轮次 | 整轮 ms | 客户端单实例均值 ms | 客户端 P50 ms | 客户端 P95 ms | 成功/请求 |
|---|---:|---:|---:|---:|---|
| blk1-R1 | 123.339 | 94.562 | 93.997 | 121.515 | 100/100 |
| blk1-R2 | 149.545 | 118.101 | 134.149 | 146.357 | 100/100 |
| blk1-R3 | 138.380 | 102.912 | 107.935 | 133.542 | 100/100 |
| blk1-R4 | 141.059 | 115.702 | 126.863 | 137.496 | 100/100 |
| blk1-R5 | 145.804 | 115.578 | 130.497 | 142.611 | 100/100 |
| blk2-R1 | 135.520 | 104.939 | 113.409 | 131.365 | 100/100 |
| blk2-R2 | 146.012 | 109.833 | 124.015 | 141.218 | 100/100 |
| blk2-R3 | 137.775 | 103.784 | 116.544 | 133.450 | 100/100 |
| blk2-R4 | 140.717 | 102.477 | 108.755 | 136.204 | 100/100 |
| blk2-R5 | 126.021 | 92.609 | 88.313 | 124.459 | 100/100 |

整轮中位 **139.549 ms**（范围 123.3–149.5，n=10）。

#### 1. 按执行顺序排列的相邻 host 区间（每格 均值 / P95 [有效实例/总实例]，ms）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Master 开始 → Cubelet 收到（分发及等待）<br>`master_start_to_cubelet_start` | 6.0 / 14.6 [103/103] | — / — [0] | 4.4 / 8.8 [103/103] | 7.4 / 13.0 [103/103] | 7.0 / 14.8 [103/103] | 7.3 / 18.9 [103/103] | 6.9 / 13.3 [103/103] | 4.8 / 9.3 [103/103] | 8.6 / 17.5 [103/103] | 7.3 / 16.2 [103/103] |
| Cubelet 收到 → 解析出快照路径（clone / cgroup 等前期准备）<br>`cubelet_start_to_resolved` | 5.3 / 9.7 [103/103] | — / — [0] | 8.5 / 16.3 [103/103] | 10.2 / 19.4 [103/103] | 8.6 / 17.9 [103/103] | 8.0 / 15.9 [103/103] | 8.1 / 13.8 [103/103] | 8.3 / 16.8 [103/103] | 7.0 / 17.7 [103/103] | 8.1 / 18.6 [103/103] |
| 快照路径已解析 → Shim 收到 Create（containerd 元数据、shim start、ttrpc）<br>`resolved_to_shim_start` | 13.9 / 23.5 [103/103] | — / — [0] | 16.9 / 27.6 [103/103] | 18.9 / 28.1 [103/103] | 16.3 / 27.2 [103/103] | 16.1 / 22.9 [103/103] | 11.6 / 17.4 [103/103] | 14.7 / 24.3 [103/103] | 13.2 / 24.4 [103/103] | 10.8 / 13.9 [103/103] |
| Shim 收到 Create → 快照初始化标记<br>`shim_start_to_snapshot_init` | 0.3 / 0.5 [103/103] | 0.4 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.4 / 0.5 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.4 / 0.5 [103/103] | 0.3 / 0.4 [103/103] | 0.4 / 0.5 [103/103] |
| 快照初始化 → 尝试加入 cgroup 后的标记<br>`snapshot_init_to_cgroup_join` | 0.9 / 1.3 [103/103] | 0.9 / 2.7 [103/103] | 0.3 / 0.9 [103/103] | 0.7 / 0.6 [103/103] | 0.3 / 0.7 [103/103] | 0.6 / 0.5 [103/103] | 1.0 / 2.0 [103/103] | 0.4 / 1.0 [103/103] | 0.6 / 0.8 [103/103] | 0.7 / 1.2 [103/103] |
| cgroup 后侧标记 → start_vm 入口<br>`cgroup_join_to_vm_start` | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] |
| start_vm → agent ready（Launch、Restore 含 prewrite、ResetVm）<br>`vm_start_to_agent_ready` | 28.4 / 39.3 [103/103] | 36.6 / 49.9 [103/103] | 30.8 / 43.8 [103/103] | 38.1 / 52.1 [103/103] | 34.3 / 49.9 [103/103] | 30.9 / 42.4 [103/103] | 36.8 / 52.7 [103/103] | 34.6 / 49.8 [103/103] | 31.8 / 45.4 [103/103] | 30.5 / 42.9 [103/103] |
| agent ready → Shim Create 返回（guest sandbox/container）<br>`agent_ready_to_shim_finish` | 3.9 / 4.7 [103/103] | 4.1 / 4.9 [103/103] | 4.0 / 4.8 [103/103] | 4.1 / 5.2 [103/103] | 4.1 / 5.1 [103/103] | 4.0 / 5.0 [103/103] | 4.0 / 4.7 [103/103] | 4.0 / 4.9 [103/103] | 3.9 / 4.8 [103/103] | 4.0 / 4.8 [103/103] |
| Shim 返回 → Master 返回（Start、探针、状态及后处理）<br>`shim_finish_to_master_reply` | 12.5 / 16.1 [103/103] | 15.5 / 24.9 [103/103] | 13.2 / 19.0 [103/103] | 12.6 / 16.9 [103/103] | 11.5 / 14.0 [103/103] | 12.6 / 19.3 [103/103] | 14.8 / 23.4 [103/103] | 13.1 / 19.9 [103/103] | 13.0 / 17.8 [103/103] | 13.5 / 21.4 [103/103] |
| Master 返回 → API created 事件<br>`master_reply_to_api_created` | 0.2 / 0.3 [103/103] | 0.3 / 0.5 [103/103] | 0.4 / 1.2 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.5 / 2.1 [103/103] | 0.2 / 0.4 [103/103] | 0.4 / 2.5 [103/103] | 0.3 / 0.4 [103/103] |

#### 2. 外层区间（包住上表若干行，不能相加）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Cubelet 收到 → Shim 收到 Create<br>`cubelet_start_to_shim_start` | 19.2 / 29.5 [103/103] | — / — [0] | 25.4 / 38.3 [103/103] | 29.1 / 39.0 [103/103] | 24.9 / 41.2 [103/103] | 24.1 / 35.4 [103/103] | 19.6 / 26.4 [103/103] | 23.0 / 32.5 [103/103] | 20.2 / 32.3 [103/103] | 18.8 / 28.3 [103/103] |
| Shim 收到 Create → start_vm 入口<br>`shim_start_to_vm_start` | 1.2 / 1.7 [103/103] | 1.2 / 3.1 [103/103] | 0.7 / 1.2 [103/103] | 1.0 / 1.0 [103/103] | 0.6 / 1.1 [103/103] | 0.9 / 0.9 [103/103] | 1.3 / 2.5 [103/103] | 0.7 / 1.4 [103/103] | 0.9 / 1.2 [103/103] | 1.1 / 1.5 [103/103] |
| Shim Create 整体<br>`shim_start_to_shim_finish` | 33.6 / 45.5 [103/103] | 41.9 / 55.6 [103/103] | 35.5 / 48.6 [103/103] | 43.2 / 57.4 [103/103] | 39.0 / 55.2 [103/103] | 35.8 / 46.8 [103/103] | 42.2 / 58.2 [103/103] | 39.3 / 54.7 [103/103] | 36.7 / 50.5 [103/103] | 35.6 / 48.1 [103/103] |
| Master 开始 → Master 返回<br>`master_start_to_master_reply` | 71.2 / 93.7 [103/103] | 92.1 / 117.8 [103/103] | 78.5 / 105.9 [103/103] | 92.3 / 116.8 [103/103] | 82.4 / 104.5 [103/103] | 79.7 / 102.0 [103/103] | 83.4 / 112.6 [103/103] | 80.4 / 107.3 [103/103] | 78.4 / 108.7 [103/103] | 75.2 / 104.4 [103/103] |

#### 3. 组件自报 stat（Cubelet <5 ms 不输出，样本为条件样本）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| cubelet.cubebox-service<br>`cubelet.cubebox-service` | 62.3 / 81.9 [103/103] | 80.5 / 102.0 [103/103] | 70.0 / 92.0 [103/103] | 81.3 / 100.5 [103/103] | 72.6 / 94.5 [103/103] | 69.8 / 91.3 [103/103] | 70.8 / 90.8 [103/103] | 71.2 / 93.6 [103/103] | 66.3 / 89.0 [103/103] | 63.2 / 81.0 [103/103] |
| cubelet.storage<br>`cubelet.storage` | 5.9 / 6.8 [25/103] | 8.7 / 13.3 [41/103] | 8.5 / 12.3 [44/103] | 9.1 / 12.6 [45/103] | 8.5 / 13.8 [42/103] | 6.5 / 8.1 [44/103] | 7.9 / 10.9 [64/103] | 8.3 / 11.6 [44/103] | 8.2 / 14.1 [37/103] | 9.7 / 14.4 [47/103] |
| cubelet.network<br>`cubelet.network` | — / — [0] | — / — [0] | — / — [0] | — / — [0] | 5.3 / 5.3 [1/103] | — / — [0] | — / — [0] | — / — [0] | — / — [0] | — / — [0] |
| cubelet.cgroup<br>`cubelet.cgroup` | — / — [0] | 8.4 / 11.7 [14/103] | 8.4 / 9.4 [2/103] | 6.6 / 7.4 [17/103] | — / — [0] | — / — [0] | — / — [0] | — / — [0] | 5.8 / 5.8 [1/103] | — / — [0] |
| cubelet.create-sandbox-metadata<br>`cubelet.create-sandbox-metadata` | 11.2 / 17.2 [58/103] | 14.0 / 23.9 [76/103] | 13.0 / 21.7 [73/103] | 12.7 / 19.9 [79/103] | 12.2 / 20.5 [71/103] | 12.1 / 18.2 [75/103] | 7.3 / 11.9 [48/103] | 9.7 / 17.2 [53/103] | 10.9 / 17.9 [51/103] | 6.6 / 8.7 [16/103] |
| cubelet.cube-sandbox-store<br>`cubelet.cube-sandbox-store` | 6.5 / 8.8 [11/103] | 11.1 / 18.8 [45/103] | 10.6 / 14.5 [24/103] | 10.2 / 12.6 [14/103] | 10.4 / 12.9 [38/103] | 9.2 / 14.6 [45/103] | 6.6 / 8.0 [7/103] | 8.5 / 12.0 [33/103] | 7.8 / 11.0 [6/103] | 5.7 / 6.1 [2/103] |
| cubelet.sandbox-create<br>`cubelet.sandbox-create` | 40.7 / 53.4 [103/103] | 49.9 / 65.5 [103/103] | 43.0 / 56.4 [103/103] | 52.0 / 66.0 [103/103] | 46.6 / 64.0 [103/103] | 42.9 / 56.1 [103/103] | 49.0 / 65.9 [103/103] | 47.8 / 64.7 [103/103] | 43.7 / 58.0 [103/103] | 42.9 / 56.1 [103/103] |
| cubelet.sandbox-start<br>`cubelet.sandbox-start` | 41.7 / 54.2 [103/103] | 50.9 / 67.3 [103/103] | 44.2 / 57.3 [103/103] | 53.0 / 67.1 [103/103] | 47.6 / 65.0 [103/103] | 44.1 / 57.0 [103/103] | 50.0 / 67.1 [103/103] | 48.9 / 65.7 [103/103] | 44.7 / 59.1 [103/103] | 43.9 / 57.0 [103/103] |
| cubelet.sandbox-probe<br>`cubelet.sandbox-probe` | 7.4 / 10.5 [101/103] | 7.1 / 8.7 [102/103] | 7.0 / 8.8 [99/103] | 7.1 / 8.4 [101/103] | 7.0 / 8.7 [100/103] | 7.6 / 12.9 [100/103] | 7.3 / 8.5 [100/103] | 7.0 / 8.4 [101/103] | 7.4 / 11.5 [101/103] | 6.9 / 8.3 [98/103] |
| cubelet.cubebox<br>`cubelet.cubebox` | 57.3 / 75.9 [103/103] | 69.9 / 92.6 [103/103] | 61.7 / 81.3 [103/103] | 71.9 / 94.6 [103/103] | 64.3 / 85.6 [103/103] | 62.1 / 79.2 [103/103] | 63.0 / 82.2 [103/103] | 63.1 / 84.1 [103/103] | 59.6 / 81.9 [103/103] | 55.4 / 70.5 [103/103] |
| shim.LaunchVmm<br>`shim.LaunchVmm` | 1.2 / 2.0 [103/103] | 1.7 / 4.0 [103/103] | 1.2 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.1 / 2.0 [103/103] | 1.3 / 2.0 [103/103] | 1.3 / 2.0 [103/103] | 1.5 / 3.0 [103/103] | 1.2 / 2.0 [103/103] | 1.4 / 2.0 [103/103] |
| shim.RestoreVm<br>`shim.RestoreVm` | 20.5 / 31.0 [103/103] | 28.2 / 42.0 [103/103] | 22.8 / 36.0 [103/103] | 30.0 / 44.0 [103/103] | 26.5 / 41.0 [103/103] | 22.8 / 34.0 [103/103] | 28.7 / 44.0 [103/103] | 26.4 / 40.0 [103/103] | 23.7 / 37.0 [103/103] | 22.3 / 34.0 [103/103] |
| shim.ResetVm<br>`shim.ResetVm` | 5.1 / 7.0 [103/103] | 5.0 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.0 / 7.0 [103/103] | 5.2 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.1 / 7.0 [103/103] | 5.1 / 7.0 [103/103] |
| shim.CreateSandbox<br>`shim.CreateSandbox` | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.2 / 1.0 [103/103] | 0.2 / 1.0 [103/103] | 0.1 / 1.0 [103/103] | 0.1 / 1.0 [103/103] |
| shim.CreateContainer<br>`shim.CreateContainer` | 2.6 / 3.0 [103/103] | 2.7 / 3.0 [103/103] | 2.7 / 4.0 [103/103] | 2.7 / 3.0 [103/103] | 2.8 / 4.0 [103/103] | 2.6 / 3.0 [103/103] | 2.6 / 3.0 [103/103] | 2.6 / 3.0 [103/103] | 2.6 / 3.0 [103/103] | 2.7 / 3.0 [103/103] |
| shim.CreatePodSandbox<br>`shim.CreatePodSandbox` | 33.1 / 45.0 [103/103] | 41.4 / 55.0 [103/103] | 34.9 / 48.0 [103/103] | 42.7 / 57.0 [103/103] | 38.5 / 55.0 [103/103] | 35.3 / 46.0 [103/103] | 41.7 / 58.0 [103/103] | 38.8 / 54.0 [103/103] | 36.2 / 50.0 [103/103] | 35.0 / 48.0 [103/103] |

#### 4. 最后完成者的关键路径（去掉 3 个串行预热；末 10% 对比中间 50%，均值 ms）

```text
FIN-BEST-blk1-c100-r1: n=100 (warm-ups dropped) last master_reply at +119.3 ms, arrivals spread 29.9 ms
FIN-BEST-blk1-c100-r3: n=100 (warm-ups dropped) last master_reply at +133.5 ms, arrivals spread 28.7 ms
FIN-BEST-blk1-c100-r4: n=100 (warm-ups dropped) last master_reply at +137.2 ms, arrivals spread 32.5 ms
FIN-BEST-blk1-c100-r5: n=100 (warm-ups dropped) last master_reply at +143.4 ms, arrivals spread 43.5 ms
FIN-BEST-blk2-c100-r1: n=100 (warm-ups dropped) last master_reply at +130.7 ms, arrivals spread 31.4 ms
FIN-BEST-blk2-c100-r2: n=100 (warm-ups dropped) last master_reply at +139.6 ms, arrivals spread 34.4 ms
FIN-BEST-blk2-c100-r3: n=100 (warm-ups dropped) last master_reply at +134.9 ms, arrivals spread 27.7 ms
FIN-BEST-blk2-c100-r4: n=100 (warm-ups dropped) last master_reply at +137.6 ms, arrivals spread 35.5 ms
FIN-BEST-blk2-c100-r5: n=100 (warm-ups dropped) last master_reply at +122.3 ms, arrivals spread 27.5 ms

segment (ms, mean)        mid 50%   slowest 10%   delta
arrival                        24.7         26.8     +2.1
master->cubelet                 7.1          8.9     +1.7
cubelet->resolved               8.7         12.3     +3.7
resolved->shim                 16.8         17.8     +1.0
shim->vm_start                  0.8          0.8     +0.0
vm_start->agent                34.0         44.0    +10.0
agent->shim_done                3.9          4.5     +0.5
shim_done->reply               12.2         16.8     +4.6
completion                    108.4        131.9    +23.5
shim.RestoreVm_ms              26.1         34.6     +8.5  (logged n=450/90)
shim.ResetVm_ms                 4.9          6.5     +1.6  (logged n=450/90)
cubelet.sandbox-probe_ms        7.0          7.9     +0.9  (logged n=448/90)
cubelet.cgroup_ms               6.7          6.9     +0.1  (logged n=13/4)
cubelet.storage_ms              8.0          8.1     +0.1  (logged n=205/72)
cubelet.create-sandbox-metadata_ms     11.6         10.7     -0.9  (logged n=326/70)
```

#### 5. OS 级阻塞（bpftrace，单独诊断轮，非计时样本）

```text
== syscalls by total time (top 15 per process kind)
cubelet  futex              total=  87882.9 ms n=   5772 avg= 15225.7 us  >1ms=594
cubelet  epoll_pwait        total=   5902.3 ms n=   6311 avg=   935.2 us  >1ms=174
cubelet  ioctl              total=    885.1 ms n=    521 avg=  1698.8 us  >1ms=211
cubelet  nanosleep          total=    779.2 ms n=   3290 avg=   236.8 us  >1ms=76
cubelet  waitid             total=    551.1 ms n=    106 avg=  5198.7 us  >1ms=106
cubelet  openat             total=     88.4 ms n=   6550 avg=    13.5 us  >1ms=0
cubelet  read               total=     70.1 ms n=   9929 avg=     7.1 us  >1ms=6
cubelet  fsync              total=     52.3 ms n=    206 avg=   253.7 us  >1ms=19
cubelet  epoll_ctl          total=     50.6 ms n=  10268 avg=     4.9 us  >1ms=0
cubelet  close              total=     49.2 ms n=   7669 avg=     6.4 us  >1ms=2
cubelet  recvfrom           total=     45.6 ms n=     27 avg=  1689.6 us  >1ms=24
cubelet  write              total=     39.5 ms n=   3131 avg=    12.6 us  >1ms=5
cubelet  clone              total=     35.1 ms n=    103 avg=   341.0 us  >1ms=0
cubelet  newfstatat         total=     22.8 ms n=   6283 avg=     3.6 us  >1ms=0
cubelet  pwrite             total=      9.8 ms n=   4822 avg=     2.0 us  >1ms=0
shim     epoll_pwait        total= 814558.3 ms n=  21084 avg= 38634.0 us  >1ms=2966
shim     futex              total= 110047.2 ms n=  46063 avg=  2389.1 us  >1ms=3465
shim     clock_nanosleep    total=    709.6 ms n=    672 avg=  1056.0 us  >1ms=672
shim     ioctl              total=    686.6 ms n= 154400 avg=     4.4 us  >1ms=102
shim     madvise            total=    292.1 ms n= 120709 avg=     2.4 us  >1ms=0
shim     write              total=    235.7 ms n=  35185 avg=     6.7 us  >1ms=18
shim     clone3             total=    204.3 ms n=   3114 avg=    65.6 us  >1ms=0
shim     prctl              total=    143.9 ms n=   4036 avg=    35.7 us  >1ms=96
shim     mprotect           total=     98.6 ms n=  29064 avg=     3.4 us  >1ms=0
shim     mmap               total=     85.5 ms n=  14738 avg=     5.8 us  >1ms=0
shim     nanosleep          total=     39.7 ms n=    557 avg=    71.2 us  >1ms=0
shim     openat             total=     37.4 ms n=   9101 avg=     4.1 us  >1ms=0
shim     read               total=     33.3 ms n=  12710 avg=     2.6 us  >1ms=4
shim     munmap             total=     24.0 ms n=   5643 avg=     4.3 us  >1ms=0
shim     execve             total=     15.6 ms n=    109 avg=   142.8 us  >1ms=0

== D-state (uninterruptible) waits by kernel stack (top 12)
cubelet  total=   185.4 ms n=    52 avg=    3565 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+172 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
cubelet  total=    94.3 ms n=    38 avg=    2482 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+240 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
shim     total=    27.1 ms n=     1 avg=   27087 us | wait_for_common+204 < wait_for_completion+32 < __wait_rcu_gp+348 < synchronize_rcu+144 < rcu_sync_enter+284 < percpu_down_write+52 < cgroup_procs_write_start+128
cubelet  total=    20.4 ms n=    53 avg=     384 us | wait_for_common+204 < wait_for_completion+32 < __flush_workqueue+292 < xlog_cil_push_now.isra.0+56 < xlog_cil_force_seq+136 < xfs_log_force_seq+164 < xfs_log_force_inode+120
shim     total=    20.2 ms n=  1231 avg=      16 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_set_irq_routing+552 < kvm_vm_ioctl+1064 < __arm64_sys_ioctl+180
shim     total=    18.7 ms n=    54 avg=     345 us | __mutex_lock.constprop.0+356 < __mutex_lock_slowpath+28 < mutex_lock+80 < cgroup_kn_lock_live+96 < __cgroup1_procs_write.constprop.0+76 < cgroup1_procs_write+28 < cgroup_file_write+164
shim     total=    13.7 ms n=   305 avg=      45 us | percpu_rwsem_wait+176 < __percpu_down_read+64 < cgroup_css_set_fork+804 < cgroup_can_fork+44 < copy_process+2760 < kernel_clone+152 < __se_sys_clone3+200
cubelet  total=    12.9 ms n=    53 avg=     243 us | xlog_wait_on_iclog+368 < xlog_force_lsn+476 < xfs_log_force_seq+204 < xfs_log_force_inode+120 < xfs_dir_fsync+48 < vfs_fsync_range+60 < __arm64_sys_fsync+72
cubelet  total=    12.4 ms n=   103 avg=     121 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone+124 < __arm64_sys_clone+40 < invoke_syscall+80
shim     total=    11.7 ms n=   102 avg=     115 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone3+200 < __arm64_sys_clone3+28 < invoke_syscall+80
shim     total=     5.4 ms n=   706 avg=       8 us | io_schedule+68 < __folio_lock_killable+340 < lock_folio_maybe_drop_mmap+148 < filemap_fault+356 < __xfs_filemap_fault+116 < xfs_filemap_fault+92 < __do_fault+68
shim     total=     5.0 ms n=   391 avg=      13 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_io_bus_register_dev+624 < kvm_assign_ioeventfd_idx+464 < kvm_ioeventfd+144
```

#### 6. 二级探针（cgroup rstat flush、reflink 源文件、Cubelet 内核态 on-CPU）

#### 7. Cubelet Go trace

Cubelet 系统调用阻塞（Go trace，3 s 窗口，按函数自身）：
```text
     1.40s 36.75% 36.75%      1.40s 36.75%  golang.org/x/sys/unix.ioctlPtr
     0.67s 17.64% 54.39%      0.67s 17.64%  github.com/tencentcloud/CubeSandbox/Cubelet/pkg/cubecow._Cfunc_cubecow_create_snapshot_from_volume_go
     0.62s 16.30% 70.69%      0.62s 16.30%  internal/syscall/unix.Waitid
     0.43s 11.38% 82.07%      0.43s 11.38%  github.com/tencentcloud/CubeSandbox/Cubelet/network/runtime.init.func4
     0.26s  6.74% 88.81%      0.26s  6.74%  github.com/tencentcloud/CubeSandbox/Cubelet/network/runtime.ioctl
     0.08s  2.16% 90.97%      0.08s  2.16%  syscall.openat
     0.08s  1.98% 92.94%      0.08s  1.98%  golang.org/x/sys/unix.recvfrom
     0.06s  1.49% 94.43%      0.06s  1.49%  syscall.Close
     0.05s  1.22% 95.66%      0.05s  1.22%  syscall.read
     0.03s  0.87% 96.53%      0.03s  0.87%  syscall.readlen
```

## C. BEST，prewrite 关

### C. BEST，prewrite 关：逐轮客户端与服务端总览

| 轮次 | 整轮 ms | 客户端单实例均值 ms | 客户端 P50 ms | 客户端 P95 ms | 成功/请求 |
|---|---:|---:|---:|---:|---|
| blk1-R1 | 157.465 | 122.179 | 130.420 | 154.394 | 100/100 |
| blk1-R2 | 159.073 | 123.641 | 134.658 | 155.164 | 100/100 |
| blk1-R3 | 153.996 | 119.359 | 128.979 | 146.667 | 100/100 |
| blk1-R4 | 150.876 | 117.668 | 128.572 | 146.558 | 100/100 |
| blk1-R5 | 151.942 | 115.154 | 118.454 | 147.504 | 100/100 |
| blk2-R1 | 156.694 | 123.683 | 125.156 | 151.221 | 100/100 |
| blk2-R2 | 164.910 | 120.023 | 122.991 | 152.774 | 100/100 |
| blk2-R3 | 160.760 | 116.125 | 124.613 | 148.783 | 100/100 |
| blk2-R4 | 149.677 | 117.708 | 116.030 | 145.994 | 100/100 |
| blk2-R5 | 157.930 | 129.289 | 140.830 | 154.755 | 100/100 |

整轮中位 **157.080 ms**（范围 149.7–164.9，n=10）。

#### 1. 按执行顺序排列的相邻 host 区间（每格 均值 / P95 [有效实例/总实例]，ms）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Master 开始 → Cubelet 收到（分发及等待）<br>`master_start_to_cubelet_start` | 5.0 / 9.3 [103/103] | 3.8 / 7.5 [103/103] | 3.5 / 6.1 [103/103] | 4.3 / 8.6 [103/103] | 5.1 / 10.3 [103/103] | 4.6 / 8.1 [103/103] | 4.9 / 9.7 [103/103] | 5.6 / 12.9 [103/103] | 5.1 / 10.3 [103/103] | 4.5 / 7.9 [103/103] |
| Cubelet 收到 → 解析出快照路径（clone / cgroup 等前期准备）<br>`cubelet_start_to_resolved` | 9.9 / 17.6 [103/103] | 8.7 / 16.8 [103/103] | 10.7 / 19.4 [103/103] | 8.9 / 17.9 [103/103] | 8.8 / 20.0 [103/103] | 11.8 / 23.9 [103/103] | 8.5 / 17.2 [103/103] | 9.4 / 20.6 [103/103] | 8.4 / 15.8 [103/103] | 9.3 / 18.2 [103/103] |
| 快照路径已解析 → Shim 收到 Create（containerd 元数据、shim start、ttrpc）<br>`resolved_to_shim_start` | 17.0 / 28.6 [103/103] | 15.6 / 23.9 [103/103] | 19.2 / 33.6 [103/103] | 14.5 / 23.1 [103/103] | 15.6 / 29.6 [103/103] | 16.7 / 32.0 [103/103] | 19.6 / 34.0 [103/103] | 18.6 / 32.8 [103/103] | 15.6 / 26.4 [103/103] | 14.4 / 23.3 [103/103] |
| Shim 收到 Create → 快照初始化标记<br>`shim_start_to_snapshot_init` | 0.3 / 0.5 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.4 [103/103] | 0.4 / 0.5 [103/103] | 0.3 / 0.5 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.5 [103/103] | 0.3 / 0.4 [103/103] | 0.3 / 0.5 [103/103] | 0.3 / 0.4 [103/103] |
| 快照初始化 → 尝试加入 cgroup 后的标记<br>`snapshot_init_to_cgroup_join` | 0.6 / 1.0 [103/103] | 1.0 / 1.5 [103/103] | 0.7 / 0.5 [103/103] | 1.3 / 1.9 [103/103] | 0.7 / 1.5 [103/103] | 0.9 / 1.9 [103/103] | 0.6 / 0.8 [103/103] | 0.7 / 1.7 [103/103] | 0.7 / 1.2 [103/103] | 1.3 / 2.5 [103/103] |
| cgroup 后侧标记 → start_vm 入口<br>`cgroup_join_to_vm_start` | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] | 0.0 / 0.0 [103/103] |
| start_vm → agent ready（Launch、Restore 含 prewrite、ResetVm）<br>`vm_start_to_agent_ready` | 34.5 / 51.7 [103/103] | 39.2 / 59.1 [103/103] | 30.7 / 44.0 [103/103] | 35.6 / 53.2 [103/103] | 29.4 / 40.4 [103/103] | 33.7 / 46.2 [103/103] | 30.5 / 42.4 [103/103] | 29.8 / 39.2 [103/103] | 33.4 / 51.2 [103/103] | 45.9 / 64.9 [103/103] |
| agent ready → Shim Create 返回（guest sandbox/container）<br>`agent_ready_to_shim_finish` | 8.1 / 13.1 [103/103] | 8.1 / 12.9 [103/103] | 7.8 / 13.0 [103/103] | 7.8 / 12.4 [103/103] | 7.8 / 13.3 [103/103] | 8.1 / 13.2 [103/103] | 7.4 / 12.3 [103/103] | 7.6 / 13.2 [103/103] | 8.4 / 13.1 [103/103] | 8.6 / 13.0 [103/103] |
| Shim 返回 → Master 返回（Start、探针、状态及后处理）<br>`shim_finish_to_master_reply` | 21.8 / 28.2 [103/103] | 21.1 / 26.8 [103/103] | 22.2 / 29.1 [103/103] | 21.8 / 28.3 [103/103] | 23.3 / 33.8 [103/103] | 22.5 / 33.1 [103/103] | 22.5 / 31.0 [103/103] | 21.6 / 27.5 [103/103] | 23.0 / 30.2 [103/103] | 20.6 / 28.0 [103/103] |
| Master 返回 → API created 事件<br>`master_reply_to_api_created` | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] | 0.2 / 0.3 [103/103] |

#### 2. 外层区间（包住上表若干行，不能相加）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Cubelet 收到 → Shim 收到 Create<br>`cubelet_start_to_shim_start` | 26.9 / 42.8 [103/103] | 24.3 / 35.6 [103/103] | 29.9 / 49.7 [103/103] | 23.4 / 34.4 [103/103] | 24.4 / 39.5 [103/103] | 28.4 / 40.9 [103/103] | 28.1 / 43.8 [103/103] | 28.0 / 45.0 [103/103] | 23.9 / 37.0 [103/103] | 23.8 / 32.7 [103/103] |
| Shim 收到 Create → start_vm 入口<br>`shim_start_to_vm_start` | 0.9 / 1.3 [103/103] | 1.4 / 1.9 [103/103] | 1.0 / 0.9 [103/103] | 1.6 / 2.2 [103/103] | 1.1 / 1.9 [103/103] | 1.3 / 2.2 [103/103] | 0.9 / 1.1 [103/103] | 1.1 / 2.1 [103/103] | 1.1 / 1.6 [103/103] | 1.6 / 3.0 [103/103] |
| Shim Create 整体<br>`shim_start_to_shim_finish` | 43.5 / 63.6 [103/103] | 48.7 / 71.4 [103/103] | 39.5 / 55.7 [103/103] | 45.0 / 66.5 [103/103] | 38.2 / 52.8 [103/103] | 43.1 / 59.8 [103/103] | 38.8 / 54.7 [103/103] | 38.5 / 51.1 [103/103] | 42.9 / 62.2 [103/103] | 56.0 / 78.3 [103/103] |
| Master 开始 → Master 返回<br>`master_start_to_master_reply` | 97.2 / 127.5 [103/103] | 97.9 / 126.6 [103/103] | 95.2 / 119.6 [103/103] | 94.5 / 121.1 [103/103] | 91.0 / 120.0 [103/103] | 98.6 / 126.2 [103/103] | 94.3 / 124.5 [103/103] | 93.7 / 122.5 [103/103] | 94.9 / 119.9 [103/103] | 104.9 / 131.2 [103/103] |

#### 3. 组件自报 stat（Cubelet <5 ms 不输出，样本为条件样本）

| 阶段 | blk1-R1 | blk1-R2 | blk1-R3 | blk1-R4 | blk1-R5 | blk2-R1 | blk2-R2 | blk2-R3 | blk2-R4 | blk2-R5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| cubelet.cubebox-service<br>`cubelet.cubebox-service` | 89.3 / 117.1 [103/103] | 91.4 / 118.8 [103/103] | 88.1 / 113.6 [103/103] | 87.3 / 112.6 [103/103] | 82.0 / 108.3 [103/103] | 90.6 / 118.1 [103/103] | 86.3 / 112.3 [103/103] | 85.6 / 111.2 [103/103] | 85.8 / 111.5 [103/103] | 97.4 / 123.7 [103/103] |
| cubelet.storage<br>`cubelet.storage` | 8.7 / 13.4 [52/103] | 8.6 / 13.3 [49/103] | 8.3 / 13.1 [48/103] | 7.8 / 12.7 [52/103] | 8.7 / 11.3 [40/103] | 10.2 / 14.5 [44/103] | 8.2 / 12.2 [42/103] | 7.6 / 11.6 [48/103] | 7.9 / 11.2 [38/103] | 9.0 / 13.4 [47/103] |
| cubelet.network<br>`cubelet.network` | — / — [0] | 9.1 / 9.1 [1/103] | 5.9 / 5.9 [1/103] | — / — [0] | — / — [0] | — / — [0] | 7.4 / 7.4 [1/103] | 13.2 / 15.0 [15/103] | — / — [0] | — / — [0] |
| cubelet.cgroup<br>`cubelet.cgroup` | 6.8 / 9.5 [7/103] | 5.7 / 6.1 [2/103] | 7.1 / 12.0 [13/103] | 7.4 / 7.9 [2/103] | 7.3 / 7.3 [1/103] | 11.5 / 11.5 [1/103] | 7.4 / 8.9 [4/103] | 6.7 / 9.6 [5/103] | 6.8 / 7.3 [4/103] | 7.3 / 8.0 [2/103] |
| cubelet.create-sandbox-metadata<br>`cubelet.create-sandbox-metadata` | 13.8 / 23.1 [72/103] | 11.2 / 19.1 [71/103] | 14.3 / 26.2 [70/103] | 9.3 / 16.9 [59/103] | 13.2 / 23.3 [62/103] | 12.5 / 25.2 [70/103] | 15.8 / 27.3 [75/103] | 17.3 / 26.7 [65/103] | 11.9 / 19.5 [68/103] | 9.1 / 17.2 [67/103] |
| cubelet.cube-sandbox-store<br>`cubelet.cube-sandbox-store` | 10.7 / 15.3 [35/103] | 10.5 / 14.5 [28/103] | 9.8 / 15.6 [61/103] | 7.9 / 11.5 [17/103] | 12.6 / 17.6 [32/103] | 15.9 / 21.5 [41/103] | 8.1 / 12.7 [22/103] | 10.1 / 18.0 [30/103] | 11.0 / 17.7 [31/103] | 8.5 / 12.7 [45/103] |
| cubelet.sandbox-create<br>`cubelet.sandbox-create` | 50.6 / 71.9 [103/103] | 56.2 / 79.3 [103/103] | 48.8 / 67.8 [103/103] | 53.3 / 75.0 [103/103] | 45.4 / 61.2 [103/103] | 50.9 / 68.8 [103/103] | 46.6 / 63.7 [103/103] | 45.7 / 58.6 [103/103] | 50.3 / 69.1 [103/103] | 63.7 / 85.1 [103/103] |
| cubelet.sandbox-start<br>`cubelet.sandbox-start` | 51.9 / 73.1 [103/103] | 57.7 / 80.3 [103/103] | 50.4 / 68.9 [103/103] | 54.5 / 76.6 [103/103] | 46.8 / 62.8 [103/103] | 52.4 / 69.8 [103/103] | 48.2 / 65.0 [103/103] | 47.2 / 59.9 [103/103] | 51.7 / 70.1 [103/103] | 65.0 / 86.7 [103/103] |
| cubelet.sandbox-probe<br>`cubelet.sandbox-probe` | 16.6 / 21.4 [103/103] | 16.0 / 20.3 [103/103] | 16.0 / 21.0 [103/103] | 16.9 / 21.7 [103/103] | 16.9 / 25.5 [103/103] | 16.6 / 25.5 [103/103] | 16.8 / 25.2 [103/103] | 16.6 / 23.1 [103/103] | 16.6 / 22.4 [103/103] | 15.5 / 21.2 [103/103] |
| cubelet.cubebox<br>`cubelet.cubebox` | 79.7 / 103.4 [103/103] | 82.9 / 105.5 [103/103] | 77.6 / 98.0 [103/103] | 78.7 / 101.6 [103/103] | 73.4 / 92.5 [103/103] | 79.0 / 104.1 [103/103] | 78.1 / 98.0 [103/103] | 76.5 / 102.3 [103/103] | 77.6 / 99.0 [103/103] | 88.3 / 112.3 [103/103] |
| shim.LaunchVmm<br>`shim.LaunchVmm` | 1.2 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.1 / 2.0 [103/103] | 1.3 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.3 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 1.3 / 2.0 [103/103] |
| shim.RestoreVm<br>`shim.RestoreVm` | 19.3 / 33.0 [103/103] | 23.7 / 41.0 [103/103] | 15.6 / 28.0 [103/103] | 21.1 / 35.0 [103/103] | 14.8 / 24.0 [103/103] | 18.6 / 29.0 [103/103] | 16.1 / 25.0 [103/103] | 15.4 / 24.0 [103/103] | 17.9 / 33.0 [103/103] | 30.3 / 47.0 [103/103] |
| shim.ResetVm<br>`shim.ResetVm` | 12.3 / 19.0 [103/103] | 12.7 / 19.0 [103/103] | 12.3 / 17.0 [103/103] | 11.5 / 18.0 [103/103] | 11.7 / 16.0 [103/103] | 12.1 / 19.0 [103/103] | 11.7 / 17.0 [103/103] | 11.4 / 16.0 [103/103] | 12.6 / 18.0 [103/103] | 12.6 / 19.0 [103/103] |
| shim.CreateSandbox<br>`shim.CreateSandbox` | 1.0 / 2.0 [103/103] | 0.9 / 2.0 [103/103] | 0.9 / 3.0 [103/103] | 0.9 / 2.0 [103/103] | 0.9 / 2.0 [103/103] | 1.2 / 2.0 [103/103] | 0.9 / 2.0 [103/103] | 0.8 / 2.0 [103/103] | 1.0 / 3.0 [103/103] | 1.2 / 4.0 [103/103] |
| shim.CreateContainer<br>`shim.CreateContainer` | 6.1 / 11.0 [103/103] | 6.2 / 10.0 [103/103] | 5.9 / 10.0 [103/103] | 5.8 / 10.0 [103/103] | 5.8 / 10.0 [103/103] | 6.0 / 10.0 [103/103] | 5.5 / 10.0 [103/103] | 5.8 / 11.0 [103/103] | 6.3 / 11.0 [103/103] | 6.3 / 10.0 [103/103] |
| shim.CreatePodSandbox<br>`shim.CreatePodSandbox` | 43.0 / 63.0 [103/103] | 48.2 / 71.0 [103/103] | 39.0 / 55.0 [103/103] | 44.5 / 66.0 [103/103] | 37.7 / 52.0 [103/103] | 42.6 / 59.0 [103/103] | 38.3 / 54.0 [103/103] | 37.9 / 51.0 [103/103] | 42.4 / 62.0 [103/103] | 55.5 / 78.0 [103/103] |

#### 4. 最后完成者的关键路径（去掉 3 个串行预热；末 10% 对比中间 50%，均值 ms）

```text
OFF-BEST-blk1-c100-r1: n=100 (warm-ups dropped) last master_reply at +154.0 ms, arrivals spread 31.1 ms
OFF-BEST-blk1-c100-r2: n=100 (warm-ups dropped) last master_reply at +154.9 ms, arrivals spread 31.9 ms
OFF-BEST-blk1-c100-r3: n=100 (warm-ups dropped) last master_reply at +151.6 ms, arrivals spread 29.6 ms
OFF-BEST-blk1-c100-r4: n=100 (warm-ups dropped) last master_reply at +147.2 ms, arrivals spread 31.1 ms
OFF-BEST-blk1-c100-r5: n=100 (warm-ups dropped) last master_reply at +148.1 ms, arrivals spread 30.6 ms
OFF-BEST-blk2-c100-r1: n=100 (warm-ups dropped) last master_reply at +153.7 ms, arrivals spread 28.7 ms
OFF-BEST-blk2-c100-r2: n=100 (warm-ups dropped) last master_reply at +161.8 ms, arrivals spread 32.5 ms
OFF-BEST-blk2-c100-r3: n=100 (warm-ups dropped) last master_reply at +157.4 ms, arrivals spread 31.6 ms
OFF-BEST-blk2-c100-r4: n=100 (warm-ups dropped) last master_reply at +146.7 ms, arrivals spread 30.0 ms
OFF-BEST-blk2-c100-r5: n=100 (warm-ups dropped) last master_reply at +141.3 ms, arrivals spread 14.9 ms

segment (ms, mean)        mid 50%   slowest 10%   delta
arrival                        23.7         24.6     +0.9
master->cubelet                 5.1          6.3     +1.2
cubelet->resolved              10.2         15.1     +4.9
resolved->shim                 19.0         21.3     +2.3
shim->vm_start                  0.8          1.1     +0.3
vm_start->agent                34.7         47.3    +12.6
agent->shim_done                7.7         10.8     +3.2
shim_done->reply               22.0         21.8     -0.2
completion                    123.2        148.4    +25.2
shim.RestoreVm_ms              20.2         28.8     +8.6  (logged n=500/100)
shim.ResetVm_ms                11.6         15.5     +3.9  (logged n=500/100)
cubelet.sandbox-probe_ms       16.4         17.1     +0.7  (logged n=500/100)
cubelet.cgroup_ms               7.2          7.7     +0.5  (logged n=21/5)
cubelet.storage_ms              8.2          9.0     +0.8  (logged n=264/79)
cubelet.network_ms             10.7         14.1     +3.4  (logged n=9/3)
cubelet.create-sandbox-metadata_ms     12.9         13.0     +0.1  (logged n=415/98)
```

#### 5. OS 级阻塞（bpftrace，单独诊断轮，非计时样本）

```text
== syscalls by total time (top 15 per process kind)
cubelet  futex              total=  24372.4 ms n=   6241 avg=  3905.2 us  >1ms=523
cubelet  epoll_pwait        total=   5903.7 ms n=   6133 avg=   962.6 us  >1ms=148
cubelet  ioctl              total=   3124.3 ms n=    521 avg=  5996.8 us  >1ms=454
cubelet  waitid             total=    560.4 ms n=    103 avg=  5440.5 us  >1ms=103
cubelet  nanosleep          total=    380.6 ms n=   4505 avg=    84.5 us  >1ms=24
cubelet  fsync              total=    118.4 ms n=    206 avg=   574.7 us  >1ms=34
cubelet  recvfrom           total=    100.4 ms n=     27 avg=  3717.6 us  >1ms=24
cubelet  openat             total=     89.2 ms n=   5609 avg=    15.9 us  >1ms=0
cubelet  epoll_ctl          total=     88.4 ms n=   8649 avg=    10.2 us  >1ms=0
cubelet  read               total=     69.7 ms n=   8837 avg=     7.9 us  >1ms=2
cubelet  close              total=     54.2 ms n=   6589 avg=     8.2 us  >1ms=0
cubelet  clone              total=     43.2 ms n=    103 avg=   419.9 us  >1ms=1
cubelet  write              total=     25.7 ms n=   3317 avg=     7.7 us  >1ms=1
cubelet  newfstatat         total=     21.5 ms n=   6040 avg=     3.6 us  >1ms=0
cubelet  pipe2              total=     17.6 ms n=    309 avg=    56.8 us  >1ms=0
shim     epoll_pwait        total= 819208.5 ms n=  22031 avg= 37184.4 us  >1ms=3399
shim     futex              total= 121795.8 ms n=  46565 avg=  2615.6 us  >1ms=4316
shim     clock_nanosleep    total=   2783.2 ms n=   2637 avg=  1055.4 us  >1ms=2637
shim     ioctl              total=   1031.1 ms n= 159032 avg=     6.5 us  >1ms=104
shim     write              total=    304.6 ms n=  36223 avg=     8.4 us  >1ms=18
shim     clone3             total=    207.7 ms n=   3190 avg=    65.1 us  >1ms=0
shim     prctl              total=    145.2 ms n=   4121 avg=    35.2 us  >1ms=99
shim     mprotect           total=    130.4 ms n=  29497 avg=     4.4 us  >1ms=0
shim     mmap               total=     81.8 ms n=  15051 avg=     5.4 us  >1ms=0
shim     nanosleep          total=     43.1 ms n=    604 avg=    71.3 us  >1ms=0
shim     openat             total=     37.2 ms n=   9143 avg=     4.1 us  >1ms=0
shim     read               total=     28.5 ms n=  12484 avg=     2.3 us  >1ms=0
shim     munmap             total=     26.0 ms n=   5755 avg=     4.5 us  >1ms=0
shim     execve             total=     14.9 ms n=    107 avg=   139.4 us  >1ms=0
shim     io_uring_setup     total=     14.6 ms n=    206 avg=    71.0 us  >1ms=0

== D-state (uninterruptible) waits by kernel stack (top 12)
cubelet  total=   282.3 ms n=    64 avg=    4411 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+172 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
cubelet  total=   147.1 ms n=    30 avg=    4902 us | rwsem_down_write_slowpath+512 < down_write+96 < xfs_iolock_two_inodes_and_break_layout+240 < xfs_ilock2_io_mmap+80 < xfs_reflink_remap_prep+92 < xfs_file_remap_range+140 < vfs_clone_file_range+288
cubelet  total=    64.0 ms n=    77 avg=     831 us | wait_for_common+204 < wait_for_completion+32 < __flush_workqueue+292 < xlog_cil_push_now.isra.0+56 < xlog_cil_force_seq+136 < xfs_log_force_seq+164 < xfs_log_force_inode+120
shim     total=    37.9 ms n=     1 avg=   37869 us | wait_for_common+204 < wait_for_completion+32 < __wait_rcu_gp+348 < synchronize_rcu+144 < rcu_sync_enter+284 < percpu_down_write+52 < cgroup_procs_write_start+128
shim     total=    23.8 ms n=    67 avg=     355 us | __mutex_lock.constprop.0+356 < __mutex_lock_slowpath+28 < mutex_lock+80 < cgroup_kn_lock_live+96 < __cgroup1_procs_write.constprop.0+76 < cgroup1_procs_write+28 < cgroup_file_write+164
cubelet  total=    20.1 ms n=    46 avg=     437 us | xlog_wait_on_iclog+368 < xlog_force_lsn+476 < xfs_log_force_seq+204 < xfs_log_force_inode+120 < xfs_dir_fsync+48 < vfs_fsync_range+60 < __arm64_sys_fsync+72
cubelet  total=    13.5 ms n=   103 avg=     131 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone+124 < __arm64_sys_clone+40 < invoke_syscall+80
shim     total=    13.0 ms n=   252 avg=      51 us | percpu_rwsem_wait+176 < __percpu_down_read+64 < cgroup_css_set_fork+804 < cgroup_can_fork+44 < copy_process+2760 < kernel_clone+152 < __se_sys_clone3+200
shim     total=    12.4 ms n=   103 avg=     121 us | wait_for_common+204 < wait_for_completion_state+32 < wait_for_vfork_done+48 < kernel_clone+360 < __se_sys_clone3+200 < __arm64_sys_clone3+28 < invoke_syscall+80
shim     total=    10.1 ms n=   875 avg=      12 us | io_schedule+68 < __folio_lock_killable+340 < lock_folio_maybe_drop_mmap+148 < filemap_fault+356 < __xfs_filemap_fault+116 < xfs_filemap_fault+92 < __do_fault+68
shim     total=     9.0 ms n=   575 avg=      16 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_set_irq_routing+552 < kvm_vm_ioctl+1064 < __arm64_sys_ioctl+180
shim     total=     7.9 ms n=   595 avg=      13 us | wait_for_common+204 < wait_for_completion+32 < __synchronize_srcu.part.0+140 < synchronize_srcu_expedited+56 < kvm_io_bus_register_dev+624 < kvm_assign_ioeventfd_idx+464 < kvm_ioeventfd+144
```

#### 6. 二级探针（cgroup rstat flush、reflink 源文件、Cubelet 内核态 on-CPU）

cgroup rstat flush（6 s 窗口；键为 是否Cubelet, comm）：1, cubelet: 14.3 ms / 10 次；0, kworker/u770:1: 7.8 ms / 1 次；0, kworker/u770:3: 0.1 ms / 1 次

| reflink 源文件 | 次数 | prep 合计 ms | 平均 ms | 最大 ms |
|---|---:|---:|---:|---:|
| `tpl-tpl-33aa75680ae14683b32f05fe-rootfs` | 103 | 85.4 | 0.83 | 3.70 |

Cubelet 内核态 on-CPU 采样（999 Hz，共 1275 个样本）前 10：
```text
   716  56.2%  osq_lock < __mutex_lock.constprop.0 < __mutex_lock_slowpath < mutex_lock < rtnl_lock
    37   2.9%  snmp6_fill_stats < inet6_fill_ifla6_attrs < inet6_fill_link_af < rtnl_fill_ifinfo < rtnl_dump_ifinfo
    31   2.4%  native_queued_spin_lock_slowpath < close_fd_get_file < __arm64_sys_close < invoke_syscall < el0_svc_common.constprop.0
    27   2.1%  osq_lock < __mutex_lock.constprop.0 < __mutex_lock_slowpath < mutex_lock < do_epoll_ctl
    23   1.8%  mutex_spin_on_owner < __mutex_lock.constprop.0 < __mutex_lock_slowpath < mutex_lock < rtnl_lock
    23   1.8%  __native_vcpu_is_preempted < __mutex_lock.constprop.0 < __mutex_lock_slowpath < mutex_lock < rtnl_lock
    22   1.7%  native_queued_spin_lock_slowpath < alloc_fd < get_unused_fd_flags < do_sys_openat2 < __arm64_sys_openat
    13   1.0%  flush_workqueue_prep_pwqs < __flush_workqueue < xlog_cil_push_now.isra.0 < xlog_cil_force_seq < xfs_log_force_seq
    12   0.9%  snmp6_fill_stats < inet6_fill_ifla6_attrs < inet6_fill_link_af < rtnl_fill_ifinfo < rtmsg_ifinfo_build_skb
    11   0.9%  dev_fetch_sw_netstats < dev_get_tstats64 < dev_get_stats < rtnl_fill_stats < rtnl_fill_ifinfo
```

#### 7. Cubelet Go trace

Cubelet 系统调用阻塞（Go trace，3 s 窗口，按函数自身）：
```text
  638.80ms 26.75% 26.75%   638.80ms 26.75%  golang.org/x/sys/unix.ioctlPtr
  535.36ms 22.41% 49.16%   535.36ms 22.41%  internal/syscall/unix.Waitid
  518.20ms 21.70% 70.86%   518.20ms 21.70%  github.com/tencentcloud/CubeSandbox/Cubelet/pkg/cubecow._Cfunc_cubecow_create_snapshot_from_volume_go
  228.78ms  9.58% 80.43%   228.78ms  9.58%  github.com/tencentcloud/CubeSandbox/Cubelet/network/runtime.init.func4
  186.61ms  7.81% 88.25%   186.61ms  7.81%  github.com/tencentcloud/CubeSandbox/Cubelet/network/runtime.ioctl
   60.74ms  2.54% 90.79%    60.74ms  2.54%  syscall.openat
   41.89ms  1.75% 92.54%    41.89ms  1.75%  syscall.read
   33.98ms  1.42% 93.97%    33.98ms  1.42%  syscall.write
   32.79ms  1.37% 95.34%    32.79ms  1.37%  syscall.Close
   27.78ms  1.16% 96.50%    27.78ms  1.16%  syscall.readlen
```

