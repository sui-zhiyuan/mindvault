# 高性能服务器多人 SSH 资源协作（numa-session）高层设计

> 更新：2026-02-13

## 简介

面向「单台高性能 Linux 服务器 + 同团队多人共用」场景的 SSH 会话管理与资源协作方案。目标是用尽量小的机制解决三件事：**独占 / 共享 / NUMA 独占三种状态的准入**、**非抢占式排队与"谁挡着我"的可沟通清单**、**批量用户的创建与 home 初始化**。核心取舍是**软绑定而非硬隔离**：独占靠 controller 记录 + 登录时自动施加 `numactl` / `taskset` 绑定，不引入 cgroup / Slurm 级别的资源硬切分——因为使用者属于同一团队，不需要防恶意与提权。本文只到高层设计，不含接口细节与实现代码。

Related: [附录 A：目标环境事实](#附录-a目标环境事实)

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| NUMA | Non-Uniform Memory Access | 非统一内存访问；一个 node = 一组 CPU + 本地内存 |
| RC | Resource Controller | 本方案的单机权威状态机 `numad` |
| Claim | — | 一次成功的占用记录（谁、占哪些资源、什么模式、到何时） |
| Holder | — | 持有某资源独占权的用户 / session |
| Lease | — | 占用有效期，到期自动释放，可续期 |
| Blocker | — | 挡住某个待满足申请的 (用户, session, 资源) 列表 |
| Session | — | 一次 SSH 登录产生的会话（`user@pts/N`） |
| MOTD | Message Of The Day | 登录时展示的公告；本项目用它广播占用与排队状态 |

## 核心内容

### 需求与设计前提

| 项 | 取值 / 决策 |
|---|---|
| 部署规模 | **纯单机**；即使有多台机器，每台也是互相独立的实例，不做跨机统一视图 |
| 强制方式 | **软绑定**：`numactl --cpunodebind/--membind` 施加 CPU 亲和与内存策略；不引入 cgroup 硬隔离 |
| 信任模型 | 同一团队，无恶意；**不防**用户自行解绑、不防提权漏洞 |
| 独占语义 | **准入控制**（拒绝新的独占申请）+ 占用者自己被绑定；不是内核级排他 |
| 资源粒度 | NUMA node（CPU + 本地内存）、整机（host）；不细分到 core / 单卡 |
| 登录落点 | 正常 shell；需要独占时用 CLI 显式申请，登录本身不被流程阻断 |
| 抢占策略 | **完全非抢占**：任何情况下不 kill 已在跑的进程；只等 holder 主动释放或 lease 到期 |

一句话架构：**一个单机权威 controller（记录谁占着什么）+ 一个登录时自动施加绑定的 launcher + 一个给人看的排队 / 让位清单**。

### 核心概念与三种模式

| 概念 | 含义 |
|---|---|
| **Resource** | `numa0..numaN`、`host`；CPU / 内存不细分的粒度就是 node |
| **Mode** | `SHARED` / `NUMA_EXCLUSIVE` / `HOST_EXCLUSIVE` |
| **Claim** | `{owner, resources, mode, session_id, acquired_at, expires_at, note}` |
| **Waiting request** | 未满足的申请：想要什么、何时开始排；同优先级严格 FIFO |
| **Blocker** | 挡住该申请的 (user, session, resource, 预计释放时间, 备注) —— 给人拿去直接沟通 |
| **Lease** | 默认 4h，可续期；到期前 15min 提醒，宽限期后自动释放 |

三种模式的语义（关键）：

| 模式 | 申请者得到 | 其他人 | 是否入队 |
|---|---|---|---|
| `SHARED` | 默认落点，不绑定；可按需用 `numa-run` 在**未被独占**的 node 上临时运行 | 不受影响 | 否 |
| `NUMA_EXCLUSIVE` | 该 node 的 cpuset + 本地内存绑定 | 仍可登录、仍在 SHARED；**不能再申请该 node**；shared 任务应避免主动跑上去（靠清单与约定自觉） | 是，按资源集 FIFO |
| `HOST_EXCLUSIVE` | 全部 node 绑定 | 新登录被 launcher 拦下（给出占用人 + 预计释放时间）；已存在的 shared 会话只提示、不 kill | 是 |

### 架构

```
                       ssh client
                           │  SSH（正常登录）
                           ▼
        ┌──────────────────────────────────────────┐
        │ sshd + pam_exec / 用户 .ssh/rc            │
        │   → 调用 numa-ctl session-enter           │
        └──────────────┬───────────────────────────┘
                       │ unix socket（0660, group=team）
                       ▼
             ┌───────────────────────────────┐
             │  numad（systemd service）      │  单机唯一权威
             │  ├ 状态机：claims / waitq      │
             │  ├ 原子写 /var/lib/numa/state  │
             │  ├ release → 派发队首          │
             │  └ MOTD / status 渲染          │
             └──────┬─────────────────┬──────┘
                    │                 │
           ┌────────▼──────┐   ┌──────▼──────────────┐
           │ numa-ctl CLI  │   │ numa-shell/launcher │
           │ request/status│   │ numactl/taskset 绑定 │
           │ release/queue │   │ 登录时由 .ssh/rc 触发│
           └───────────────┘   └─────────────────────┘
```

- **`numad`**：唯一状态持有者，无数据库；JSON + `fsync` + `rename` 原子落盘，重启即恢复。
- **`numa-ctl`**：用户可见 CLI，走同一 socket；操作幂等，可安全重放。
- **launcher**：登录时报告 session、取回该用户已有 claim 并施加绑定；`numactl` 缺失时降级为 `taskset -c`（只绑 CPU，不绑内存）。

### 关键流程

**A. 普通登录**

1. sshd → `$HOME/.ssh/rc` → `numa-ctl session-enter $USER $SSH_CONNECTION`。
2. `numad` 记录 session；若该用户有活跃 claim，返回 `numa=<N>`。
3. rc 设置 `NUMA_NODE=N`，并 `exec numactl --cpunodebind=$NUMA_NODE --membind=$NUMA_NODE <真 shell>`。
4. 无 claim → 直接进 shell（SHARED）；MOTD 展示占用概览与自己的排队位置。

**B. 申请独占（非抢占 + 阻塞清单）**

```
$ numa-ctl request --numa 1,2 --for 4h --note "跑 A/B benchmark"
requested: numa1,numa2 (NUMA_EXCLUSIVE, 4h)
[blocked] numa2 held by alice@pts/3 since 13:20, lease ends 17:20
          contact: matrix @alice
→ queued at position 1 (blocked by 1 user)
```

- 多资源是**全有或全无**：只满足一半不授予，避免"占着空闲 node 反而堵别人"。
- 每次状态变化（release / 到期）`numad` 重新派发队首；被授予者须在 60s 内 `numa-ctl accept <id>`，否则顺延，防止僵尸占位。
- 已在 SHARED 里跑着任务的人拿到独占后，需要重新登录或 `numa-run --claim <id>` 让新进程树继承绑定；**已在跑的进程不会被自动迁移**——这是软绑定的固有代价，必须在 README 里写明。

**C. 让位与沟通（本设计的重点能力）**

```
$ numa-ctl blockers
you want: numa2 (queued 14:02, position 1)
  blocker  user    session      since   lease-end  last-seen  note
  numa2    alice   alice@pts/3  13:20   17:20      2m ago     A/B benchmark
  numa2    bob     bob@pts/7    15:01   none       12m ago    (shared, soft)
earliest possible free: 17:20
```

- 附带 **最早可能空闲时间**，让等待者判断"等还是改需求"。
- 联系方式从 roster 文件读取；`numa-ctl poke <user>` 向对方终端写一条本地消息，不依赖邮件 / IM 基础设施。
- `--json` 输出供脚本消费（例如自动重试直到拿到资源）。

**D. 释放**

| 路径 | 行为 |
|---|---|
| 主动 `numa-ctl release` | 立即释放并派发队首 |
| 登出 | `session-enter` 的配对清理自动释放 |
| lease 到期 | 到期前 15min 提醒 → 宽限 10min → 自动释放并标记 `expired` |
| 异常掉线 | 心跳 `last-seen` 超时且进程不存在 → 判 stale 释放 |

### 数据模型

```jsonc
// /var/lib/numa/state.json
{
  "version": 1,
  "nodes": { "numa0": {"cpus": "0-15", "mem_mb": 65536}, "numa1": {"cpus": "16-31", "mem_mb": 65536} },
  "claims": [
    {"id": "c-8f2", "mode": "NUMA_EXCLUSIVE", "resources": ["numa2"],
     "owner": "alice", "session": "alice@pts/3",
     "acquired": "2026-02-13T13:20:00+08:00", "expires": "2026-02-13T17:20:00+08:00",
     "note": "A/B benchmark", "state": "active"}
  ],
  "waitq": [
    {"id": "w-31", "owner": "carol", "mode": "NUMA_EXCLUSIVE", "resources": ["numa1", "numa2"],
     "requested": "2026-02-13T14:02:00+08:00", "note": "多节点 scaling 测试",
     "state": "waiting"}
  ],
  "sessions": [
    {"id": "alice@pts/3", "user": "alice", "tty": "pts/3",
     "since": "2026-02-13T13:19:40+08:00", "last_seen": "2026-02-13T15:41:00+08:00", "claim": "c-8f2"}
  ]
}
```

**不变量**：任一资源同一时刻最多一个独占 claim；`HOST_EXCLUSIVE` 与任何其他独占互斥；`numad` 启动时重算状态并打印修复日志（用 `logind` / `ss` 校正孤儿 session）。

### 用户管理与 home 初始化

**清单即真相**：`/etc/numa/users.yaml`（示例结构）

```yaml
defaults:
  shell: /usr/local/bin/numa-shell     # 薄 wrapper：施加绑定后 exec 真 shell
  groups: [team]
users:
  alice:
    full_name: Alice Zhang
    ssh_keys: files/alice.pub
    home_paths: ["/opt/tools/bin", "/usr/local/cuda/bin"]
    template: dev                      # 决定 README 与环境变量模板
    contact: "matrix @alice"
```

`numa-ctl users` 子命令全部幂等：

| 命令 | 行为 |
|---|---|
| `users add alice` | `useradd` + 安装 `authorized_keys` + 渲染 home 模板 + 写 `/etc/profile.d/10-numa-<user>.sh` |
| `users sync [--all]` | 只重渲染带 `# managed-by-numa` 标记的块，**不动用户自己的文件**；模板版本记在 `~/.numa/template.version` |
| `users del alice [--purge-home]` | 先释放其全部 claim → `userdel` → home 归档到 `/var/backups/numa/alice-<date>.tar.zst`（默认保留，不直接删） |
| `users ls` | 用户 + 是否在线 + 当前 claim |

首次登录自动初始化的内容：

- `~/.numa/README.md`：三分钟上手 —— 三种模式、`request` / `blockers` / `release` 用法、为什么不要自己解绑、常见算例。
- `/etc/profile.d/10-numa.sh`：`PATH`、`NUMA_NODES`、`numa-ctl` 补全；MOTD 打印当前占用表。
- `numa-ctl doctor`：自检 `numactl` 是否存在、当前绑定是否生效、node 容量是否符合预期。

### CLI 面（草案）

```
numa-ctl status [--json]                   # 全机占用 + 队列
numa-ctl request --numa 1,2 | --host --for 4h [--note ...] [--wait]
numa-ctl blockers [--json]                 # 谁挡着我 + 联系方式 + 最早空闲
numa-ctl cancel                            # 撤销排队
numa-ctl accept <id> | release [<id>]
numa-ctl poke <user> [msg]                 # 向对方终端写本地消息
numa-ctl numa-run --numa 3 -- <cmd...>     # 在未被独占的 node 上临时运行命令
numa-ctl users add | sync | del | ls
numa-ctl doctor
```

### 失效处理与运维

| 场景 | 处理 |
|---|---|
| `numad` 崩溃 | 状态已原子落盘；重启加载 + 用 `logind` / `ss` 校正孤儿 session |
| session 异常掉线 | 心跳超时且无进程 → 标 stale 并释放 |
| lease 到期 | 提醒 → 宽限 → 自动释放；**不做 kill** |
| 用户忘了释放 | 每次登录 MOTD 提醒，`status` 中标黄 |
| 事后复盘 | 追加式 `events.log` 记录"谁在何时申请 / 让位 / 释放"，仅作审计，非安全用途 |

### 实现落点（最小可跑）

1. `numad`：单进程守护 + Unix socket，核心状态机约数百行（Rust/Tokio 或 Python asyncio）。
2. `numa-ctl`：同仓库第二个可执行文件，纯客户端。
3. 系统集成：`numad.service`、`/etc/numa/users.yaml`、`/etc/profile.d/10-numa.sh`、`/usr/local/bin/numa-shell`、可选的 `pam_exec`（覆盖非 bash 登录）、`sudoers` 中给 `numa-ctl users` 的受限条目。
4. 依赖：`numactl`（必需）、`taskset`（降级路径）、`python3`（模板渲染）。**无需** libcgroup、Slurm、数据库。

### 设计取舍（为什么这么简单）

- **不做硬隔离**：团队内互信，`numactl` 绑定 + 准入控制足以覆盖"我跑 benchmark 时别人别来抢"的真实需求；cgroup cpuset 会带来 slice 生命周期、登录耦合、迁移等大量复杂度。
- **不做抢占**：抢占需要判断"谁的任务更重要"，在无优先级体系的团队里只会引发争议；改为等待队列 + 可直接沟通的 blockers 清单。
- **不做跨机调度**：多机场景下每台机器独立，符合"机器之间互不相关"的实际使用方式，也避免分布式一致性。
- **状态极简**：一个 JSON 文件即全部真相，重启、排障、人工干预都只需读一个文件。

## Q&A

**Q：软绑定下，用户自己 `taskset` 改回来怎么办？**
A：设计上不防——前提是团队互信。真正需要保证的场景（例如对外提供性能数字）应改用 cgroup cpuset 硬隔离，那是另一档复杂度。

**Q：为什么多资源申请要全有或全无？**
A：部分授予会让等待者占住一部分资源，反而挡住其他本可以完整使用的人，且让"最早空闲时间"的估算更混乱。

**Q：独占期间其他人还能登录吗？**
A：NUMA 独占下能（仍在 SHARED，只是不能再申请该 node）；只有 `HOST_EXCLUSIVE` 会拦新登录，且不 kill 已有会话。

**Q：已经在跑的进程会随登录被绑定吗？**
A：不会。绑定作用于登录后的新进程树，已有进程保持原状，属于软绑定的已知限制。

## 参考资料

- `man numactl`、`man taskset`（CPU 亲和与内存策略）
- `man systemd.exec`、`man pam_exec`（登录集成方式）
- [mdBook 文档](https://rust-lang.github.io/mdBook/)（本仓库文档组织方式）
- 访问日期：2026-02-13

## 附录 A：目标环境事实

开发机实测（Ubuntu 24.04.4 LTS、systemd 255）：

| 能力 | 状态 |
|---|---|
| `systemd-run` | 存在 |
| `taskset` | 存在 |
| `numactl` | **未安装**（需先装 `numactl` 包） |
| `cgcreate` / `cgset` | 未安装（本设计不需要） |
| `pam_exec` | `command -v` 未命中（属 PAM 模块，需按发行版路径检查） |
| `slurmd` / `srun` | 未安装（本设计不依赖） |
| 本机 NUMA 拓扑 | 单 node（`numa0`, CPU 0-19）——绑定逻辑的功能验证需在真实多 node 机器上进行 |

> 说明：以上为环境探测结果，用于确认工具链可用性，不代表目标生产机配置。

## 附录 B：待确认问题

1. **SHARED 会话是否给默认绑定**：倾向不绑（完全自由）；若希望 shared 用户别乱跑到即将被独占的 node，可改为默认钉在 `numa0`。
2. **是否需要硬保证**：当前按软绑定设计；若必须"别人物理上用不了"，需引入 cgroup cpuset。
3. **排队公平性**：倾向严格 FIFO + 可视化 blockers，不引入优先级。
4. **`HOST_EXCLUSIVE` 是否拦登录**：可改为"降级为只读 SHARED 会话"（能登录、能沟通，但重任务无绑定）。
