# SSH Session / NUMA 占用协调工具设计

> 更新：2026-09-18

## 简介

一台多 NUMA 的高性能 Linux 服务器由同一团队多人共用时，缺少"谁在用哪半台机器"的共识，
性能测试会被邻居的内存带宽争用污染。本文给出一个 **无守护进程** 的 CLI 工具的高层设计：
以共享状态目录 + 文件锁记录 NUMA node 的占用声明（claim），在 ssh 登录时用 `numactl`
自动把会话绑到应得的 node，并提供"谁挡着我 / 我占到几点"的沟通闭环。
协作是 **非抢占** 的：申请独占后进入 drain 等待，现有会话跑完自然让出。
信任模型为团队内互信——不防绕过、不防提权，设计目标是尽可能简单。

前置：了解 NUMA 基本概念与 `numactl`。
相关：[高性能服务器多人 SSH 资源协作（numa-session）高层设计](numa-ssh-multiuser-design.md)
是同一需求的 **守护进程 + lease 变体**；本文是在"无 daemon、无租约超时、惰性回收"
约束下重做的简化变体。另见 [Kunpeng 920B 硬件规格](../arm-kunpeng/kunpeng-920b-specs.md)。

## 术语列表

| Term | Full Name | Meaning |
| --- | --- | --- |
| NUMA | Non-Uniform Memory Access | 非一致内存访问；本设计中唯一的资源分配单位 |
| claim | — | 某用户对一组 NUMA node 的排他性声明，有 waiting/draining/held 三态 |
| session | — | 一次交互式 ssh 登录，记录其绑定的 node |
| drain | — | 标记 node 不再接纳新会话，等待存量会话自然退出 |
| reconcile | — | 每次 CLI 调用开头执行的状态收敛（惰性回收） |
| FIFO | First In First Out | 按 claim 创建时间的全局排队顺序 |
| GC | Garbage Collection | 此处指失效 session / claim 的清理 |

## 核心内容

### 定位与信任模型

- **是什么**：一个 CLI（下文暂名 `slot`，避免与 coreutils 的 `mv` 冲突），把"谁占了哪个
  NUMA"写进共享状态文件，并在登录时自动完成绑定。
- **不是什么**：不是作业调度器（不代替用户执行任务），不是安全边界。
- **约束手段**：登录钩子 + `numactl` 绑定 + 显式提示。用户手工 `numactl --all` 越界属于
  沟通问题，工具不做拦截，也不考虑用户覆盖配置的情况。

### 资源模型：三种情况统一为一种 claim

唯一分配单位是 NUMA node（枚举自 `/sys/devices/system/node/node*`）。

| 模式 | 表达 | 语义 |
| --- | --- | --- |
| 共享 | 无 claim | 登录时落到共享池中会话数最少的 node，内存策略 `--preferred` |
| NUMA 独占 | claim over `{2}` 或 `{2,3}` | 该 node 只绑给 owner，他人不再被分配进来 |
| 整机独占 | claim over 全部 node | NUMA 独占的特例 |

> 关键简化：整机独占 = 申请所有 node，不引入第三套逻辑，代码里只有一种 claim。

### 状态布局

```text
/srv/slot/                       # 目录 setgid，group=hpc，0770
├── lock                         # flock 目标，所有写操作串行化
├── state.json                   # 单一真相，写临时文件 + rename 原子替换
├── events.log                   # JSONL 追加：request/grant/release/reap/login
└── config/                      # 公开 git repo 的本地 clone（只读使用）
    └── users.toml
```

`state.json` 骨架：

```json
{
  "version": 1,
  "boot_id": "…",
  "claims": [{
    "id": "c-a1b2",
    "user": "user-a",
    "nodes": [2, 3],
    "state": "waiting | draining | held | stale",
    "created": "2026-09-18T09:12:03+08:00",
    "note": "跑 benchmark，预计 20:00 前释放",
    "contact": "user-a@example.com / IM: user-a"
  }],
  "sessions": [{
    "id": "s-3f77",
    "user": "user-b",
    "pid": 48211,
    "pid_starttime": 918273,
    "tty": "pts/5",
    "from": "192.168.1.23",
    "nodes": [0],
    "started": "2026-09-18T08:41:10+08:00"
  }]
}
```

- **session 身份**：优先用 logind session id；否则用登录 shell 的 PID 加
  `/proc/<pid>/stat` 的 starttime（字段 22），防止 PID 复用导致误判存活。
- **claim 与 session 解耦**：owner 断线重连不丢独占（长任务常态）。
- **无 timer**：一切回收发生在 `reconcile()` 内，由下一次 CLI 调用驱动。

### reconcile：惰性收敛

每次 CLI 调用的第一步，持 `flock` 执行：

1. 删除 PID 已死或 starttime 不匹配的 session；
2. 对每个 `waiting` claim：若它是需要这些 node 的 **最早** 等待者，把 node 标为 `draining`；
3. 若 draining 的 node 上已无他人 live session → claim 升为 `held`；
4. 若某 `held` claim 的 owner 长时间没有任何 live session → 标为 `stale`（只标记不自动删），
   任何人可执行 `slot reap` 并在 `events.log` 留痕。

### 非抢占等待（drain）

```bash
slot take --nodes 2,3 --note "跑到 20:00"
#  → 创建 claim(waiting) → node 2,3 进入 draining
#  → draining node 不再接受新 session（已在其上的会话不受影响，继续跑完）
#  → 他们退出后，下一次任意 CLI 调用把 claim 提升为 held
slot wait          # 前台轮询，实时打印"还差谁"
```

- **公平性与防死锁**：按 claim 创建时间全局 FIFO；只有最早等待者才有权 drain，
  避免两个请求互相 drain 形成死锁。
- **代价**：drain 期间该 node 空转，是非抢占的必然成本；`status` 中显式标注空转时长，
  便于人工决定是否撤销申请。

### 沟通闭环：谁挡着我

```text
$ slot blockers --nodes 2,3
node 2  user-b  pts/5  from 192.168.1.23  since 08:41  procs 12  note: "调试，随时可停"  IM: user-b
node 3  user-c  pts/9  from 192.168.1.40  since 昨天    procs 3   note: —                IM: user-c
```

输出直接带上 `users.toml` 里的联系方式和对方登记的 note。反向也成立：
holder 的 `slot status` 会显示"user-a 正在等你的 node 2（已等 12 分钟）"。
`slot note` 可随时更新自己的占用说明，他人登录时即可看到占用人与预计释放时间。

### 登录时自动绑定

`/etc/profile.d/99-slot.sh`，仅对交互式登录 shell 生效，用环境变量 `SLOT_BOUND` 防递归：

```bash
slot session enter        # 注册 session，返回应绑定的 node
exec numactl --cpunodebind=N --membind=N -- "$SHELL" -l
```

分配策略：

1. 自己有 `held` claim → 绑到自己的 node，内存用 `--membind`（硬绑，防跨 node 访问污染测量）；
2. 否则 → 共享池（非 held、非 draining）中会话数最少的 node，内存用 `--preferred`
   （避免内存打满直接 OOM）；
3. 共享池为空（全被独占）→ 不绑定，打印醒目警告与 `blockers` 列表，建议只做轻量操作。
   **不拒绝登录**，保留救火通道。

登录时同时打印 `slot status --brief` 作为 MOTD 概览。

### CLI 界面

```text
slot status [--brief]            # 每 node：状态 / 占用人 / note / 会话数 / 等待队列
slot take --nodes 2,3 | --all  [--note "…"]
slot wait  [--claim ID]          # 阻塞等待，打印剩余阻塞者
slot blockers [--nodes …]
slot release [--claim ID]
slot note "预计 20:00 释放"       # 更新自己的占用说明
slot sessions                    # 所有 live session 及其绑定
slot reap <claim-id>             # 清理 stale claim，写 events.log

slot-admin sync [--dry-run]      # root：按 git 中的 users.toml 同步账号
slot-admin doctor                # root：校验状态目录权限、NUMA 拓扑、钩子安装
```

### 用户管理：git 单一配置文件

用户真相是一个公开可访问 git repo 中的单文件：

```toml
# users.toml
[[user]]
name    = "user-a"
uid     = 3001            # 固定 uid，重装/换机保持 home 归属一致
comment = "Team member A"
contact = "user-a@example.com / IM: user-a"
state   = "active"        # active | disabled —— 删除用软状态，不删行
ssh_keys = ["ssh-ed25519 AAAA…"]
```

`slot-admin sync` 以 root 运行，**幂等**、支持 `--dry-run`：

1. `git -C /srv/slot/config pull`；
2. 新增 → `useradd -u <uid> -G hpc`，按 skeleton 建 home；
3. 变更 → 同步 `comment` / `authorized_keys` / 附加组；
4. `state = disabled` → `usermod -L`、移出 `hpc` 组、home 打包到 `/srv/archive/`
   （**不直接删除**，保守可回滚）；
5. 结果写入 `events.log`。

home skeleton（`/etc/skel.hpc/`）内容：

- `README.md`：机器规格、NUMA 拓扑、`slot` 用法速查、协作约定、成员联系方式表；
- 预建 `bin/`、`.local/bin/`，`~/.profile.d/` 片段注入 PATH 与共享路径
  （如 `/srv/data`、`/opt/toolchains`）；
- `.bashrc` / `.tmux.conf` 片段：提示符中显示当前绑定的 NUMA node。

## 延伸

### 已知边界与取舍

| 事项 | 说明 |
| --- | --- |
| tmux/screen 继承旧绑定 | attach 到已有 tmux server 的新 pane 继承 server 启动时的绑定。建议提供 `slot shell`（按 node 使用独立 tmux socket），或独占变更后重启 tmux server；需在 README 中明写 |
| 非交互 ssh（scp/rsync/git） | 不走 `profile.d`，不绑定也不注册 session。视为噪声，可接受 |
| 无守护进程的时效性 | 状态只在有人调用 CLI 时收敛；`status` 输出自带"最后 reconcile 时间"。`slot wait` 的轮询本身在持续驱动收敛 |
| 惰性回收残留 | 断电或 OOM 杀掉登录 shell 后 claim 变 `stale`，需人工 `reap`（有留痕，互信环境足够） |
| `--membind` 的风险 | 独占模式下内存硬绑，超限即 OOM 而非跨 node 溢出；这是刻意选择，保证独占者不被邻居内存带宽干扰 |

### 实现选型

单个 Python 3 stdlib 脚本（约 600 行）+ 一个 `profile.d` 片段 + 一个安装脚本，
无第三方依赖，便于在离线机器上部署。NUMA 拓扑读 `/sys`，无需解析 `lscpu` 输出。

### 可选增强

- **cpuset cgroup 替代 numactl**：绑定更硬、可做 CPU/内存用量统计，
  代价是 perf、驱动类调试可能受限，且侵入性上升。
- **只读看板**：`slot status --json` 加一个静态页，便于在团队群里贴链接。
- **多机扩展**：状态目录放到共享存储并加机器维度；复杂度显著上升，当前范围外。

## Q&A

**为什么不用 Slurm？** Slurm 面向批量作业排队，而本场景要的是交互式 ssh 会话下的
"谁占了哪半台机器"共识；引入调度器会改变团队的日常使用方式，成本远高于收益。

**为什么不做硬隔离？** 团队内互信，无恶意破坏假设。软绑定足以消除"误踩"这一真实问题，
而 cgroup 全托管会干扰 perf、驱动调试等常见工作。

**申请独占后需要等多久？** 取决于存量会话何时退出，工具不设上限，也不抢占；
`slot blockers` 给出阻塞者与联系方式，剩下的靠人沟通。

**独占者退出登录会丢掉独占吗？** 不会。claim 与 session 解耦，长任务断线重连仍持有；
只有长时间没有任何 live session 才被标为 `stale`，且需他人显式 `reap`。

## 参考资料

- [numactl(8) 手册](https://man7.org/linux/man-pages/man8/numactl.8.html) — 访问于 2026-09-18
- [Linux NUMA sysfs 接口（Documentation/ABI/stable/sysfs-devices-node）](https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-devices-node) — 访问于 2026-09-18
- [flock(2) 手册](https://man7.org/linux/man-pages/man2/flock.2.html) — 访问于 2026-09-18
- [proc(5) 手册（/proc/pid/stat 字段 22 starttime）](https://man7.org/linux/man-pages/man5/proc.5.html) — 访问于 2026-09-18
