# Linux top：内存列改造与输出解读

> 更新：2026-09-22

## 简介

`top` 是 Linux 上最常用的实时进程监控工具，但它的默认输出有两类高频误读：一是 `%MEM` 只是比例，想直接看"实际占了多少内存"必须换成 `RES` 并调整单位；二是汇总区（头部）五行与任务区各列口径不同，`free`、`VIRT`、`%CPU`、`load` 都极容易看反。

本文以 **procps-ng 4.0.4**（Ubuntu 24.04 自带）为准，先给出"把 `%MEM` 改成绝对内存占用"的完整做法，再逐行、逐列解释含义，最后记录线程视图、进程状态 `S`/`D`/`T`、以及四种 UID 这几个最常被问到的概念。

Prerequisites: 需使用 **procps-ng**（Debian/Ubuntu、RHEL 8+ 自带）；busybox 的 `top` 不支持 `-e`/`-E` 等选项。

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| RES | Resident Memory Size | 常驻物理内存（未被换出），`RES = RSan + RSfd + RSsh`（Linux 4.5+） |
| VIRT | Virtual Memory Size | 进程的虚拟地址空间总量，含已映射未使用、已换出的部分 |
| SHR | Shared Memory Size | RES 中可能被其他进程共享的子集（共享匿名页 + 共享文件映射 + 程序镜像/共享库） |
| USS | Unique Set Size | 完全不与其他进程共享的常驻内存，来自 `smaps_rollup` |
| PSS | Proportional Set Size | 共享页按共享进程数均摊后的常驻内存，多进程求和不会重复计 |
| TID | Thread ID | 线程（task）的调度 ID，见于 `/proc/<pid>/task/<tid>/` |
| TGID | Thread Group ID | 线程组 ID，等于该组 leader 的 TID，即用户眼中的进程 PID |
| RUID | Real User ID | 启动该进程的真实身份 |
| EUID | Effective User ID | 权限检查所依据的身份，`top` 的 USER 列显示的就是它 |
| FSUID | File System User ID | 文件系统访问检查实际使用的 uid，通常跟随 EUID |

## 核心内容

### 1. `%MEM` 的本质与"改成实际内存占用"

`top` **没有**"把 `%MEM` 列直接换成绝对字节"的开关——`%MEM` 的定义就是比例。`man top` 原文：

> `%MEM - simply RES divided by total physical memory`

正确做法是：**隐藏 `%MEM`，改看 `RES`（必要时加 `SHR`/`SWAP`），并把内存单位从 KiB 缩放成 MiB/GiB**。

#### 命令行一次性生效

```bash
top -o RES -e m -E m -w
#   -o RES  按 RES 排序（按实际占用排序就该用它，而不是 %MEM）
#   -e m    任务区内存（RES/VIRT/SWAP/CODE/DATA…）按 MiB 显示
#   -E m    顶部汇总区（MiB Mem / MiB Swap）也按 MiB
#   -w      加宽，避免 RES/COMMAND 被截断
```

单位取值：`k`(KiB，默认) `m` `g` `t` `p`（汇总区还多一个 `e`=EiB）。

实测效果：

```
    PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
    945 user      20   0   10.5g 729.6m  71.7m S   0.0   4.6  30:30.02 node
    354 user      20   0   71.2g 302.0m  74.2m S   0.0   1.9  46:56.64 opencode
```

`RES` 变成带后缀的绝对量（`729.6m`、`10.5g`），`%MEM` 依旧是百分比。这就是"改成实际占用"最接近的效果。

#### 交互式改完并永久保存

| 按键 | 作用 |
|---|---|
| `e` | 循环切换**任务区**内存单位（k→m→g→t→p） |
| `E` | 循环切换**汇总区**内存单位 |
| `f` | 进入字段管理：选中 `%MEM` 按 `d`(或空格) **隐藏**；选中 `RES`/`SHR`/`SWAP` 按 `d` 启用；`s` 设为排序键；`→`/`←` 调列序；回车或 `q` 返回 |
| `W` | **大写** W，把当前所有开关写进 `~/.toprc`，下次启动即默认生效 |

想更精确，可在 `f` 里启用 `USS`、`PSS`、`RSan`/`RSfd`/`RSsh` 等列（需读 `smaps`，开销约为普通内存统计的 10 倍，且查看他人进程需要 root）。

### 2. 汇总区（头部）逐行解读

样例输出：

```
top - 10:00:39 up 5 days, 23:45,  1 user,  load average: 0.16, 0.06, 0.02
Tasks:  61 total,   1 running,  60 sleeping,   0 stopped,   0 zombie
%Cpu(s):  0.0 us,  0.0 sy,  0.0 ni,100.0 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st
MiB Mem :  15845.2 total,  12836.2 free,   2310.6 used,    944.7 buff/cache
MiB Swap:   4096.0 total,   4096.0 free,      0.0 used.  13534.6 avail Mem
```

#### 第 1 行：时间 / 开机时长 / 登录用户数 / 负载

| 字段 | 含义 | 注意 |
|---|---|---|
| `10:00:39` | 当前时间 | |
| `up 5 days, 23:45` | 开机至今 | |
| `1 user` | **登录用户数**（utmp） | 不是进程数，开多个终端仍可能只算 1 个用户 |
| `load average` | 最近 1 / 5 / 15 分钟平均负载 | 见下 |

**load 的坑**：它是「**可运行(R) + 不可中断睡眠(D)**」任务数的指数衰减平均，不只是"忙不忙"。磁盘/NFS 卡住一批 `D` 状态进程时，CPU 很闲 load 也会飙高。
判据：`load ÷ 逻辑核数`（`nproc`）持续 > 1 才算排队。8 核 load=8 是满负荷。

#### 第 2 行：任务状态统计

| 字段 | 内核状态 | 含义 |
|---|---|---|
| `total` | — | 任务总数；开线程模式（`H`）后是**线程**总数 |
| `running` | R | 在 run-queue 上。man 原文强调：更准确的理解是**"已就绪"**，不代表此刻正在 CPU 上执行 |
| `sleeping` | S（+D） | 可中断睡眠；**`D`（不可中断睡眠）也被计入这一栏** |
| `stopped` | T / t | 被作业控制信号暂停，或被调试器暂停 |
| `zombie` | Z | 已退出但父进程尚未 `wait()` 回收 |

所以 `sleeping` 数字大完全正常，守护进程平时就是 S；僵尸进程长期存在才是问题。

#### 第 3 行：CPU 百分比

| 字段 | 含义 |
|---|---|
| `us` | 用户态时间（未调 nice 的进程） |
| `sy` | 内核态时间；**还包含给 guest 虚拟机跑的 vcpu 时间** |
| `ni` | 调整过 nice 值的用户态时间 |
| `id` | 空闲 |
| `wa` | 等待 I/O 完成；**高 wa 指向存储/网络瓶颈，不是 CPU 瓶颈** |
| `hi` / `si` | 硬中断 / 软中断（后者网络收包多时明显） |
| `st` | 被 hypervisor 偷走的时间（虚拟机被宿主或其他 VM 抢占） |

**最关键的一点：汇总行是所有 CPU 归一化到 100%（跨核平均），而任务区 `%CPU` 默认是 Irix 模式、可超过 100%。两者口径不同，不能互相对数。**

实测（20 核机器，只跑满 1 个核）：

```
%Cpu(s):  4.9 us,  1.0 sy,  0.0 ni, 94.2 id, ...      ← 100/20 ≈ 5
2971941 user  20  0  7952 3040 2880 R 100.0  0.0 0:02.40 bash
```

任务区 100.0%，汇总行 us 约 5.0%。

相关按键：

- `1`：把每个 CPU 拆成单独一行；`2`：按 NUMA 节点分组；
- `I`（大写 i）：切到 Solaris 模式，任务区 `%CPU` 也除以核数（最多 100%）；
- `t`：在几种 CPU 显示样式间循环（可切到 `%Cpu(s): 75.0/25.0 100[...]` 条形图）。

#### 第 4 / 5 行：内存与 Swap

| 字段 | 数据来源 | 含义 |
|---|---|---|
| `total` | MemTotal | 可用物理内存总量（不含内核保留/坏页） |
| `free` | MemFree | 完全未被使用的 |
| `used` | **MemTotal − MemAvailable** | procps 4.x 的算法；老版本是 `total − free − buff/cache` |
| `buff/cache` | Buffers + Cached + SReclaimable | 页缓存 + 可回收 slab，**需要时随时回收** |
| `avail Mem` | MemAvailable（内核 3.14+） | 估算"不触发 swap 就能给新程序用"的量 |
| Swap `total/free/used` | SwapTotal/SwapFree | 换出空间 |

两个结论：

1. **判断内存紧不紧张看 `avail Mem`，不要看 `free`。** Linux 把空闲内存拿去做缓存是设计行为，`free` 长期偏小是高效状态。
2. `avail Mem` 印在 Swap 那一行末尾只是排版位置，它是**物理内存**指标，与 swap 无关。

> 单位由大写 `E` 循环切换，命令行对应 `top -E m`。

### 3. 任务列表各列含义

默认 12 列：`PID USER PR NI VIRT RES SHR S %CPU %MEM TIME+ COMMAND`

| 列 | 含义 | 陷阱 / 备注 |
|---|---|---|
| **PID** | 进程 ID（开线程视图后每行是线程，此列实为 TID） | `top -p 1234` 只看指定进程 |
| **USER** | **有效用户**名（EUID），不是 RUID | `-u` 按 EUID 过滤，`-U` 匹配 RUID/EUID/SUID/FSUID 任意一个 |
| **PR** | 调度优先级；普通进程显示 `20 + NI`，实时进程显示 `rt` | `rt` = SCHED_FIFO/RR，优先级高于所有普通进程 |
| **NI** | nice 值，范围 −20(最高)…19(最低) | 0 表示不调整 |
| **VIRT** | 虚拟地址空间总量：代码 + 数据 + 共享库 + 已换出 + **已 mmap 但从未触碰**的页 | **几乎无参考价值**，glibc arena、预留映射、驱动/GPU 映射会把它撑到几十 GB，大 ≠ 占内存 |
| **RES** | 常驻物理内存（未被换出） | **"实际内存占用"最常用的口径** |
| **SHR** | RES 中可能被其他进程共享的部分，**也含程序镜像与共享库**（内核把它们记为共享） | 估算"独占量"看 `RES − SHR` 的量级，精确值用 `USS`/`PSS` |
| **S** | 状态：`R` 运行 / `S` 睡眠 / `D` 不可中断 / `I` 空闲 / `T` 作业控制停止 / `t` 被调试器停止 / `Z` 僵尸 | 详见核心内容第 5、6 节 |
| **%CPU** | **自上次屏幕刷新以来的间隔内**占用的 CPU 时间比 | 不是开机以来平均（那是 `%CUU`）。多线程未开 `-H` 时可 >100%；`I` 切 Solaris 模式归一化 |
| **%MEM** | `RES ÷ 物理内存总量` | 多进程共享库重复计数 |
| **TIME+** | 该任务**累计消耗的 CPU 时间**（分:秒.百分秒），非墙钟时间 | 跑了一周只占 30 秒 CPU 的守护进程很正常 |
| **COMMAND** | 程序名；按 `c` 切到完整命令行；内核线程显示为 `[kthreadd]` | 列宽不足时末尾 `+` 表示**被截断**，用 `-w` 加宽或按 `5c` 滚屏查看 |

#### 可按 `f` 打开的常用列

| 列 | 含义 |
|---|---|
| `SWAP` | 被换出的部分 |
| `USED` | `RES + SWAP`，任务实际"要过"的物理内存总量 |
| `CODE` / `DATA` | 可执行代码驻留量 (TRS) / 私有数据+栈 (DRS)，后者含尚未落到 RES 的部分 |
| `USS` | **Unique Set Size**：不与其他任何进程共享的常驻内存，最接近"这进程独占了这么多" |
| `PSS` | **Proportional Set Size**：共享页按共享进程数均摊，多进程求和不会重复计 |
| `RSan` / `RSfd` / `RSsh` | RES 的三个组成部分：私有匿名 / 文件映射(程序+共享库) / 显式共享内存 |
| `nTH` | 线程数 |
| `TTY` / `WCHAN` / `ENVIRON` / `CGROUPS` / `%CUU` | 控制终端 / 内核等待函数(需 root) / 环境变量 / cgroup / 生命周期平均 CPU 占用 |

### 4. `-H` 线程视图：它显示的是**所有线程**，且内存列会重复

先纠正一个常见误解：**不存在"后台线程"这个概念**。`man top` 原文：

> `-H, --threads-show` — Instructs top to display individual threads. Without this command-line option a summation of all threads in each process is shown.

即：默认模式把进程中**所有线程**的 CPU 时间汇总成一行；开了 `-H` 才让**每个线程**（含主线程）各占一行。

#### 原理：内核里没有"进程 vs 线程"，只有 task

每个线程就是一个 `task_struct`，区别只在**是否共享 `mm`（地址空间）**：

| | 线程之间**共享** | 线程之间**独立** |
|---|---|---|
| 资源 | 地址空间 mm、文件描述符表、cwd/umask、uid/gid、信号处置、cgroup | **TID**、栈、寄存器/调度实体、信号掩码、errno、nice/优先级 |

线程组由 **thread group leader** 领头，其 `TID == PID == TGID`；其他线程的 TID 各异，位于 `/proc/<pid>/task/<tid>/`。

所以在 `-H` 模式下：

- **`PID` 列里装的其实是 TID**，主线程那行的数字恰好等于进程 PID；
- `COMMAND` 列显示的是**线程名**（`pthread_setname_np()`/`prctl(PR_SET_NAME)` 可单独设置），这就是 `HeapHel+`、`HTTP Cl+`、Java 的 `GC Thread#0` 之类名称的来源；
- `%CPU` / `TIME+` 是**每个线程各自的**，因此 `-H` 是定位"哪个线程在烧 CPU"的标准手段。

#### ⚠️ 内存列是重复的：不要把线程 RES 相加

实测（某 17 线程进程）：

```
    PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
    354 user      20   0   71.2g 309356  76000 S   0.0   1.9  32:04.02 opencode
    374 user      20   0   71.2g 309356  76000 S   0.0   1.9   6:04.37 opencode
    416 user      20   0   71.2g 309356  76000 S   0.0   1.9   0:00.12 Bun Poo+
```

所有 17 行的 `RES` 均为 **309356 KiB 同一个值**（`sort -u` 只剩一个），`%MEM` 全为 1.9%。原因是线程共享同一个 `mm`，而 VIRT/RES/SHR/%MEM 是**地址空间属性**，不是线程属性。

**结论：`-H` 模式下把同一进程各线程的 RES 相加，会按线程数放大约 N 倍。** 要看进程级内存就关掉 `-H`。

相关命令：

```bash
top -H -p 354                            # 只看一个进程的所有线程
ps -L -p 354 -o pid,tid,comm,pcpu        # 等价的线程列表
cat /proc/354/task/*/comm                # 纯文本线程名
cat /proc/354/task/374/stack             # 需 root，看该线程的内核栈
```

### 5. 可中断睡眠 S 与不可中断睡眠 D

| | **S**（`TASK_INTERRUPTIBLE`） | **D**（`TASK_UNINTERRUPTIBLE`） |
|---|---|---|
| 在等什么 | 等一个**事件**：socket 数据、pipe、定时器、终端输入、子进程退出、锁的慢路径 | 等**内核必须做完、不能半途而废**的底层操作：块设备 I/O 完成、驱动寄存器、DMA、部分内核锁、NFS RPC |
| 信号能否唤醒 | **能**。任何未阻塞信号立刻把它唤醒到 R 去执行信号处理函数 | **不能**。信号只记在 pending 里，直到回到可中断状态才处理 |
| `Ctrl-C` / `kill -9` | 立即生效 | **不立即生效**；`kill -9` 也会一直挂着，直到 I/O 返回、内核让出该段代码，进程才真正死掉 |
| 计入 load average | **不计** | **计入** ← load 高但 CPU 全闲就是它造成的 |
| 典型时长 | 可任意长（守护进程常年 S，正常） | 通常毫秒级；存储/NFS/虚拟化磁盘异常时可达数分钟甚至永久（"**D 状态卡死**"） |
| 见到意味着 | 正常 | 存储 / 驱动 / 网络文件系统有状况 |

内核细节：

1. 两者是 `task_struct->state` 中**不同的位**：`TASK_INTERRUPTIBLE = 1`，`TASK_UNINTERRUPTIBLE = 2`。
2. 两个变体：
   - `TASK_KILLABLE = TASK_UNINTERRUPTIBLE | TASK_WAKEKILL`：**只有致命信号**（SIGKILL）能打断，NFS 常用；`top` 同样显示为 `D`。
   - `TASK_IDLE = TASK_UNINTERRUPTIBLE | TASK_NOLOAD`：状态同 D 但**不计入 load**，`top` 显示为 `I`。
3. **为什么必须有 D**：内核在修改文件系统元数据、写驱动寄存器、配置 DMA 的中途被打断会留下不一致状态，因此这段代码刻意屏蔽信号——这是"正确性优先于响应性"的取舍。

排查命令：

```bash
ps -eo pid,stat,wchan:32,comm | awk '$2 ~ /^D/'
ps -o pid,stat,wchan:32,comm -p <PID>   # wchan = 内核里卡在哪个函数
cat /proc/<PID>/stack                   # 需 root，直接看内核调用栈
cat /proc/<PID>/wchan
echo w > /proc/sysrq-trigger            # 把所有 D 任务的栈 dump 到 dmesg
```

> 回顾：`top` 第 2 行的 `sleeping` 计数**把 D 也算进去了**，该栏大不代表有问题。

### 6. stopped：Ctrl-Z 与调试器

`S` 列区分两个字母：

| 显示 | 内核状态 | 谁造成的 |
|---|---|---|
| `T` | stopped by **job control signal** | `SIGSTOP`、**`SIGTSTP`（Ctrl-Z 发的就是它）**、`SIGTTIN`/`SIGTTOU`（后台进程读写终端） |
| `t` | stopped by **debugger during trace** | `ptrace` 停住：gdb 断点、strace/ltrace attach、`SIGTRAP` |

- **Ctrl-Z = 给前台进程组发 `SIGTSTP`**，得到 `T`。shell 里 `jobs` 可见，`fg` / `bg` / `kill -CONT` 恢复（`SIGCONT`）。
- `SIGSTOP` 与 `SIGTSTP` 的区别：`SIGSTOP` **不可捕获、不可忽略、不可阻塞**；`SIGTSTP` **可被程序捕获**，所以 vim/less 能先恢复终端设置再挂起。
- `t` 是调试场景：gdb 里中断程序、单步、命中断点时进程即处于该状态，父进程通常是 gdb/strace，用 `ps -o pid,ppid,stat,comm` 即可区分。

两者共同点（易被忽略）：

- 被停住的任务**从调度器里彻底摘出**，**完全不消耗 CPU**，`TIME+` 冻结；
- 但**内存（RES/SHR）照占**、**打开的文件与 socket 照持有**。Ctrl-Z 一个大文件导入进程，它会一直攥着内存和数据库连接——这正是 `nohup` / `&` / `disown` 存在的理由；
- `kill -9` 对 T/t 状态**有效**（它们可被中断），会正常退出或变成 Z。

**不要和 `Z`(zombie) 混淆**：Z 是已死、只等父进程 `wait()` 回收的壳，`kill` 无效，要治的是父进程；T/t 是**活着的**，一个 `SIGCONT` 就能复活。

### 7. RUID / EUID / SAVED / FSUID

`/proc/<PID>/status` 的 `Uid:` 行实际是**四个** uid：

```
Uid:  1000   1000   1000   1000
      │      │      │      └── fsuid  (file system uid)
      │      │      └───────── saved set-user-ID
      │      └──────────────── effective uid
      └─────────────────────── real uid
```

| UID | 含义 | 用途 |
|---|---|---|
| **RUID** | **谁启动了它**，登录身份 | 记账、审计、历史遗留的信号权限判断 |
| **EUID** | **当下以什么身份行事** | **权限检查的主依据**：访问文件、发信号、ptrace、能力判定 |
| **SUID**（saved set-user-ID） | setuid 程序留下的"原身份备份" | 让程序能在 real/effective/saved **三者之间**来回切换（不允许随意设成新值，否则提权失控） |
| **FSUID** | Linux 专有，**文件系统访问检查实际使用的是它** | 平时跟随 EUID，NFS/容器场景可独立设置 |

#### 经典例子：`/usr/bin/passwd`

```
-rwsr-xr-x 1 root root 64152 /usr/bin/passwd     ← s 即 setuid 位
```

执行时：`RUID = 用户`，`EUID = 0`，因此它能写 `/etc/shadow`。程序内部只在真正改密码那一刻用 `EUID=0`，写完立即 `seteuid(用户)` 切回——最小权限原则，避免全程持 root 权限被利用。

`ps` 里 `RUSER` 与 `EUSER` 不一致，就是这个信号：

```bash
ps -o pid,ruser,euser,ruid,euid,suid,fsuid,cmd -p <PID>
#   PID RUSER    EUSER     RUID  EUID  SUID FSUID COMMAND
#  1234 user     root      1000     0     0     0 passwd
```

#### `sudo` 与 setuid 是两种不同机制

- `sudo` 二进制**本身也是** `-rwsr-xr-x root root` 的 setuid 程序，启动瞬间 `RUID=1000 / EUID=0`；
- 但它接着**主动**把 real / effective / saved **全部**设成目标用户，所以：

```bash
$ sudo id
uid=0(root) gid=0(root) groups=...     ← RUID 也是 0，不是 1000
```

而 setuid 二进制只改 EUID，RUID 保持原用户。这就是二者的本质差别：**setuid 是"借用身份"，sudo 是"切换身份"。**

#### 落到工具上

- `top` 的 **`USER` 列显示的是 effective user name**（man 原文：*"The effective user name of the task's owner."*）；
- `-u` 按 **EUID** 过滤，`-U` 匹配 **real / effective / saved / fs 任意一个**；
- egid/sgid/fsgid 与补充组同理；线程层面 Linux 理论上允许每线程持有独立凭证，但 POSIX 要求进程级一致，glibc 会同步到所有线程。

验证命令：

```bash
grep -E '^(Uid|Gid)' /proc/<PID>/status
ps -o pid,ruser,euser,user,ruid,euid,suid,fsuid,cmd -p <PID>
```

## 延伸

### 按键速查

| 键 | 作用 |
|---|---|
| `l` / `t` / `m` | 分别开关 头部第 1 行(load) / CPU 行 / 内存两行 |
| `1` / `2` | 每核单独一行 / 按 NUMA 节点分组 |
| `c` | 命令行 ↔ 程序名 |
| `H` | 进程 ↔ 线程视图 |
| `i` | 隐藏空闲进程 |
| `u` | 只看某用户；`k` 杀进程；`r` renice |
| `e` / `E` | 任务区 / 汇总区内存单位缩放 |
| `f` | 字段管理（增删列、排序键、列序） |
| `s` | 改刷新间隔；`W`（大写）保存到 `~/.toprc` |
| `z` / `b` / `x` | 颜色 / 加粗 / 高亮排序列 |

### 读 top 最常踩的 5 个坑

1. **`free` 小 ≠ 内存不足** —— 看 `avail Mem`。
2. **`VIRT` 大 ≠ 内存泄漏** —— 看 `RES`，要精确看 `PSS`/`USS`。
3. **各进程 `RES` 之和 ≠ 汇总 `used`** —— 共享页被重复计算，内核线程与缓存未计入。
4. **`load` 高 ≠ CPU 忙** —— 含 `D` 状态，可能是 I/O 卡住。
5. **汇总 `%Cpu(s)` 与任务 `%CPU` 口径不同** —— 前者跨核归一化到 100%，后者默认 Irix 模式可超 100%。

### 常用替代/补充工具

```bash
smem -k                                  # USS / PSS 视角，多进程不重复计
ps -o pid,comm,rss,vsz -p <PID>          # 脚本取数，单位 KiB
top -b -n1 -o RES -e m | head -20        # 批量快照
grep -E 'VmRSS|VmSwap|RssAnon|RssFile|RssShmem' /proc/<PID>/status
cat /proc/<PID>/smaps_rollup             # Pss/Rss 的权威来源
nproc                                    # 与 load 对比判据
```

## Q&A

**Q1：能不能让 `top` 直接把 `%MEM` 显示成绝对内存？**
不能。`%MEM` 按定义就是 `RES / 物理内存总量`。做法是隐藏该列、改看 `RES`，并用 `-e m`（或交互 `e`）把单位缩放到 MiB/GiB。

**Q2：`-H` 显示的是后台线程吗？**
不是。它显示该进程的**全部线程**（含主线程），只是把默认的"按进程汇总"改成"按线程分行"。而且线程行里的 `RES`/`VIRT`/`%MEM` 是整进程的同一个值，不能相加。

**Q3：为什么 `kill -9` 杀不掉某个进程？**
大概率它处于 `D`（`TASK_UNINTERRUPTIBLE`）状态，正卡在存储/驱动的不可中断代码里。信号会挂起，等 I/O 返回后进程才死。用 `ps -o stat,wchan:32` 或 `cat /proc/<PID>/stack`（root）定位。

**Q4：`top` 里 `T` 状态一定是 Ctrl-Z 造成的吗？**
不全是。`T` 来自作业控制信号（`SIGSTOP`/`SIGTSTP`/`SIGTTIN`/`SIGTTOU`），Ctrl-Z 只是最常见的一种；被 gdb/strace 停住则显示小写 `t`。

**Q5：一个 setuid 程序在 `ps` 里会是什么样？**
`RUSER` 是启动者、`EUSER` 是目标身份（通常是 root），这就是 `top` 的 `USER` 列（显示 EUID）与 `ps` 的 `RUSER` 不一致的原因。

## 参考资料

- `man top`（procps-ng 4.0.4，Ubuntu 24.04）—— SUMMARY Display、DESCRIPTIONS of Fields、Linux Memory Types 各节，访问日期 2026-09-22
- [top(1) — man7.org](https://man7.org/linux/man-pages/man1/top.1.html)
- [proc(5) — man7.org](https://man7.org/linux/man-pages/man5/proc.5.html)（`/proc/<pid>/status` 的 `Uid:` 四元组、`State` 字段）
- [The /proc Filesystem — Linux Kernel Documentation](https://www.kernel.org/doc/html/latest/filesystems/proc.html)（`smaps_rollup`、`wchan`）
- [credentials(7) — man7.org](https://man7.org/linux/man-pages/man7/credentials.7.html)（real/effective/saved/fs uid 语义）
