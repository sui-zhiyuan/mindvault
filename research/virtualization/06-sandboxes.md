# 沙箱（sandbox）全景 —— 与虚拟化的关系与对照

> 更新：2026-09-16 ｜ 类型：**中间稿（06 号）**，非最终汇总稿
> 关联：`00-index.md`（调研提纲）、`07-existing-notes-map.md`（存量笔记映射，**本文直接引用不复写**）、
> `08-verification-notes.md`（高风险项验证）、`09-synthesis-and-trends.md`（综合与趋势）
> 引用纪律：性能数字一律附**测试条件 + 出处**；本轮未取得可靠来源的，逐条写「**未能证实**」，不做估算填充。
> 本文件不修改 `docs/` 下任何内容；`docs/rust-kunpeng/` 三份沙箱笔记仅作为来源引用。

---

## 核心结论

1. **沙箱是目的，虚拟化是手段之一。** 沙箱回答"把不可信代码关进哪里"，虚拟化只是其中一类隔离边界的实现方式。本仓库的 `07-existing-notes-map.md` 已把这层关系固化为：CubeSandbox / AgentENV 属于"**硬件辅助虚拟机级**沙箱"，是虚拟化的**消费者**而非替代品。

2. **按隔离边界分层是唯一自洽的分类轴。** 共享宿主内核（④）→ 用户态内核（③）→ 独立 guest 内核（②）→ 硬件 TCB（①），隔离强度单调上升，代价（启动延迟、内存开销、运维复杂度）也单调上升。⑤ 应用/语言级和 ⑥ 网络/数据级则是**正交维度**——它们可以叠加在任意一层之上，也可以独立存在。

3. **"共享内核"是安全分水岭。** 只要 guest 与宿主共享同一个内核，沙箱的强度就**不可能超过宿主内核自身的漏洞状况**。Google 的 gVisor 官方文档把这写在设计前提下：沙箱存在的意义就是"沙箱内的程序根本无法直接对宿主发起系统调用"（源码层拦截）。

4. **有出处的量级数字（关键对照）：** Firecracker 官方 SPECIFICATION 承诺 `1 vCPU / 128 MiB` 配置下 VMM 线程内存开销 `≤ 5 MiB`、从 `InstanceStart` 到 guest `/sbin/init` 起 `≤ 125 ms`，测试环境为 M5D.metal（关闭超线程）与 M6G.metal；openEuler StratoVirt 官方写 microvm 机型 `50 ms 内启动`、内存占用 `< 4 MB`、运行时系统调用数 `< 46`；腾讯云 CubeSandbox 官方 README 写 `<60 ms` 冷启动、每实例内存开销 `<5 MB`，而**其自家 benchmark 报告（BMI5 裸金属 / Xeon 8255C / 2vCPU+2GiB 沙箱）实测串行创建 avg `47.8 ms`、50 并发 avg `276.1 ms`、每 VM 摊销内存 `~21.5 MB @100 实例`** —— **两份官方材料的并发延迟与内存口径相差约 4 倍，必须并列引用**（详见 ② 档脚注）。

5. **gVisor 的 syscall 结构性开销极大，且强依赖平台选择。** 官方 `syscall.csv`（n1-standard-4 Broadwell，Debian 9，4.19 内核）给出：runc `1939 ns`、runsc-ptrace `38219 ns`（约 **19.7 倍**）、runsc-KVM `763 ns`。**同一份数据同时说明"gVisor 一定慢"是错的**——KVM 平台下该微基准甚至快于 runc。真正决定成败的是平台，而非"gVisor"这个名字。

6. **gVisor 的固定内存开销约 20 MiB 量级。** 同一官方密度实验：`sleep` 空容器 runc `4,092,150` B vs runsc `23,695,032` B，**净增约 19.6 MiB**；`node` 应用 `76.7 MB` vs `124.1 MB`；`ruby` `45.7 MB` vs `106.1 MB`；`redis` 约 1 GB 数据时 `1055 MB` vs `1077 MB`（此时被数据本身淹没）。

7. **容器比 microVM 快，但没有数量级差距。** SOSP'17《My VM is Lighter (and Safer) than your Container》在同一台机器上实测：`fork/exec` 约 `1 ms`、unikernel VM 启动 `4 ms`、**Docker 容器启动约 `150 ms`**；LightVM 保存 `~30 ms` / 恢复 `~20 ms`，标准 Xen 为 `128 ms` / `550 ms`。这说明"VM 比容器慢一个数量级"是**旧印象**，瓶颈原本在 XenStore 与设备创建的软件路径上，可被工程手段消除。

8. **AI Agent 场景把性能诉求的排序改了。** 传统云原生优化"稳态吞吐"，Agent 沙箱优化**启动延迟 / 并发实例数 / pause-resume 时间 / 内存占用**（本仓库 `sandbox_research.md` 已把这一优先级列为待与合作方确认的核心问题）。snapshot/fork 从"锦上添花"变成**一等公民**：CubeSandbox 提供百毫秒级 snapshot/clone/rollback，AgentENV 走内存 CoW 并支持 fork 出子沙箱。

9. **microVM 的快照能力是 hypervisor 架构的直接红利。** KVM 把 guest 物理内存做成 VMM 进程的普通用户态映射，于是"给运行中的 VM 做内存快照/克隆"退化为"对进程地址空间做页级 CoW"。换成 Xen dom0 或 Hyper-V 微内核模型，这条路径不成立。**CubeSandbox 的 CoW 在文件系统层（XFS reflink），AgentENV 的 CoW 在虚拟化层（内存）**——两条路线的差异已在 `07` 号稿中固化，本文只引用。

10. **DSH（DeepSeek Harness）自身的沙箱属于 ④ 档，且是"同世界约束"（same-world confinement）。** 其 `@deepseek-ai/dsh-sandbox` 包 README 原文即写明：进程"仍然共享宿主内核与文件系统；当需要隔离整个环境时，应改用容器、microVM 或远程执行器"。这与 ②/③ 档方案**不是同一类东西**，差距是结构性的而非调参可补。

11. **DSH 沙箱没有任何内核强制的网络出口控制。** 其 bwrap profile 只包含 `--unshare-pid`，**不含 `--unshare-net`**；Landlock 启动器的授权也仅覆盖文件路径。网络出口治理在 `@deepseek-ai/dsh-http-proxy` 一层，靠进程级 HTTP 代理环境变量实现，且**只覆盖走 `fetch`/dispatcher 的调用**（该包 README 自述 E2B SDK 与 OTLP exporter 因为自带 transport 而绕过了代理）。

12. **"容器 = 沙箱"在工程上是错的，有权威 CVE 佐证。** NVD 对 CVE-2019-5736 的描述：runc 至 1.0-rc6（Docker 18.09.2 之前）允许攻击者**覆盖宿主 runc 二进制并取得宿主 root**；CVE-2022-0492 描述：内核 cgroup v1 `release_agent` 特性可提权并**绕过 namespace 隔离**；CVE-2024-21626（GHSA-xr7r-f8xq-vfvv）描述：runc 1.1.11 及更早版本因内部 fd 泄漏，使新 spawn 的容器进程工作目录落在**宿主文件系统命名空间**内，构成容器逃逸。

13. **机密计算（① 档）不是"绝对安全"，它的信任根被攻破过，而且是设计上就被排除在威胁模型外的那类攻击。** SGX 的威胁模型**明确排除侧信道**；SGAxe / CacheOut（CVE-2020-0549）**从 Intel 签名的 quoting enclave 提取了 attestation 私钥并伪造 quote**；CacheWarp（CVE-2023-20592）与 ÆPIC Leak（CVE-2022-21233）都是**架构性**（非瞬态执行）缺陷；BadRAM（IEEE S&P'25）仅需物理接触内存条 SPD 芯片即可**攻破 SEV-SNP 的 attestation**（TDX 与 Scalable SGX 有别名检测，目前不受影响）。SEV 系列官方明确把可用性攻击、侧信道、物理攻击、信任锚被攻破列为 out of scope。**做 Agent 沙箱时选 ① 档通常是合规驱动，而非安全驱动。**

14. **DSH 与同类 Agent 编码工具的差距精确地落在"网络出口"这一处。** Claude Code 在 Linux 上用 **bubblewrap（文件系统）+ socat（把流量中继到沙箱代理做域名级网络隔离）+ 可选 seccomp**；DSH 的 bwrap profile 只有 `--unshare-pid`、**无 `--unshare-net`**，出口控制退到进程级 HTTP 代理且**自带 transport 的调用可绕过**（DSH 自己的 README 承认 E2B SDK 与 OTLP exporter 就绕过了）。这是 DSH 沙箱最值得补、且补法有现成参考的一块。

---

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| SGX | Software Guard Extensions | Intel 在 CPU 内隔离 enclave 的机密计算能力 |
| TDX | Trust Domain Extensions | Intel 的机密虚拟机（TD）能力 |
| SEV / SEV-ES / SEV-SNP | Secure Encrypted Virtualization (Encrypted State / Secure Nested Paging) | AMD 的虚拟机内存加密与完整性保护能力 |
| CCA / RME | Confidential Compute Architecture / Realm Management Extension | Armv9-A 的 Realm 隔离世界 |
| TEE | Trusted Execution Environment | 通用术语：与富操作系统并存的受信执行环境（TrustZone 的 Secure world 即其一） |
| MicroVM | Micro Virtual Machine | 极简设备模型的轻量虚拟机（Firecracker 提出） |
| VMM | Virtual Machine Monitor | 用户态虚拟机监视器（QEMU / Firecracker / crosvm / Cloud Hypervisor） |
| LSM | Linux Security Module | 内核安全钩子框架（Landlock / AppArmor / SELinux 均属此类） |
| CoW | Copy-on-Write | 写时复制，快照/克隆的通用实现机制 |
| SBPL | Sandbox Profile Language | macOS Seatbelt（`sandbox-exec`）的策略语言 |

---

## 类型学（按**隔离边界**分层）

选档判据只有一句：**不可信代码与宿主之间隔了几个"能拒绝它"的边界？** 每档给出：隔离机制 / 代表产品 / 启动延迟量级 / 内存与性能开销 / 已知逃逸风险 / 适用场景。

### ① 硬件级 / 机密计算（Hardware TCB / Confidential Computing）

| 项 | 内容 |
|---|---|
| **隔离机制** | 把信任边界收缩到 CPU 内部（或 CPU + 固件/微码），宿主 OS、hypervisor、甚至云管理员都在边界**之外**；数据在内存中以密文存在，由内存控制器加解密 |
| **启动延迟量级** | 与 ② 档同量级（机密 VM 仍是 VM，额外成本在密钥管理/度量，而非引导）。**相对裸机的启动与吞吐开销百分比本轮未取得可核验数字 —— 未能证实**（找到的 SIGMETRICS'25 实证论文 PDF 无法抓取核验） |
| **内存与性能开销** | **有一手数字的一项**：Intel TDX 的 PAMT（physical-address-metadata table）在 TDX module 初始化时静态分配，约 **1/256 系统内存**（内核 dmesg 示例 `262668 KBs allocated for PAMT`；按 TDX module 版本实测约 **0.4%** 系统内存）；**Dynamic PAMT** 特性可把静态分配降到约 **0.004%**（4K 级用 1 bit/4K 的 page bitmap 替代）。其余运行时开销百分比 —— **未能证实**。注意：机密计算的开销与"沙箱效率"是**两件不同的事**，不可与 microVM 的 5 MiB 直接比较 |
| **已知逃逸风险** | 主要威胁**不是**"逃逸到宿主"（宿主本就不在 TCB 内），而是：**侧信道**（缓存/功耗/写模式）、**架构性故障注入**、**宿主对 guest 的可用性攻击**（拒绝服务、饿死）、以及**信任锚被物理攻破**。SCA 类攻击尤其关键——SGX 的威胁模型**明确把侧信道排除在范围外**，而这恰是历史上多次实际攻破的入口 |
| **适用场景** | 多租户云端处理敏感数据、跨机构联合计算、需要向客户证明"连云厂商也看不到明文"的场景 |

**分项说明（本轮已核实，全部附出处）：**

- **Intel SGX** — Intel 官方表述：SGX 在 Xeon 上可用，"通过在 CPU 内的安全 enclave 中隔离关键代码与数据来保护使用中的数据"，自称提供"数据中心中最小的信任边界：只有运行在安全 enclave 内的代码可以访问敏感数据，其他所有软件与管理员都被阻止"，配套隔离、加密与**远程证明**。粒度是**进程内的 enclave**。
  [Intel SGX](https://www.intel.com/content/www/us/en/architecture-and-technology/software-guard-extensions.html)（2026-09-16）
  - **EPC 容量随世代剧变**（条件 = CPU 世代 + SKU + BIOS/固件 + 插槽数）：第 3 代之前 Xeon 可扩展最大 **256 MB**；第 3 代每插槽最多 **512 GB**、双路 **1 TB**；第 4 代部分 SKU 512 GB、部分 128 GB；第 5 代 512/128/64 GB；Xeon 6 每插槽 512 GB、双路 1 TB。早期 SGXv1（2015，第 6 代 Core 起）EPC 仅约 **128 MB** 且所有 enclave 共享、创建后不能动态加页。[Intel 000059614](https://www.intel.com/content/www/us/en/support/articles/000059614/processors/intel-xeon-processors.html)、[SoK arXiv:2512.22090](https://ar5iv.labs.arxiv.org/html/2512.22090)
  - **消费级已弃用、服务器级仍在**：Intel 官方称 SGX 自第 11 代 Core 起不再支持（影响 UHD Blu-ray DRM 播放）；但 Xeon 6 继续支持，EPC 可达 TB 级。[Intel 000089271](https://www.intel.cn/content/www/cn/zh/support/articles/000089271/intel-nuc.html)
  - **实际攻破案例**：Plundervolt（CVE-2019-11157，S&P'20，用软件欠压接口注入故障破坏 enclave 完整性与 attestation，需 untrusted OS 的 root，Intel 以微码+BIOS 关闭欠压接口缓解）[plundervolt.com](https://plundervolt.com/)；SGAxe / CacheOut（CVE-2020-0549，**从 Intel 签名的 quoting enclave 中提取 attestation 私钥并伪造 quote**，在侧信道缓解全开、系统被视为 trusted 时仍成功；演示条件为对执行 RSA 解密的用户程序做 L1D eviction 采样，采样约 40 s + 恢复私钥约 10 s）[sgaxe.com](https://sgaxe.com/)；ÆPIC Leak（CVE-2022-21233，USENIX Sec'22，APIC MMIO 未定义区间返回 cache 层 stale 数据，属**架构性**泄露而非瞬态执行，主要影响 SGX）[aepicleak.com](https://aepicleak.com/)
- **Intel TDX** — 信任边界是 **TD（硬件隔离的 VM，而非进程内 enclave）**；TDX module 运行于新增 CPU 模式 **SEAM**（Secure Arbitration Mode），位于 SEAMRR 指定的保留内存，作为 VMM 的 peer。架构元素含 SEAM、GPA 的 **shared bit**、secure EPT、physical-address-metadata table、**TME-MK**（Total Memory Encryption – Multi-Key）与远程证明。
  [Intel TDX 概览](https://www.intel.com/content/www/us/en/developer/tools/trust-domain-extensions/overview.html)（2026-09-16）
  - **内核侧细节**：用 MKTME 私有 KeyID 加密 guest 内存，BIOS 划分 KeyID；host 不能直接访问 guest 寄存器/内存，MMIO 改为 guest 内 `#VE` 处理以避免向 host 暴露寄存器状态；私有/共享内存由 guest 自己的页表位控制；TDX **无法在 S3 及更深睡眠态存活**，与休眠互斥。[Linux 内核 TDX 文档](https://docs.kernel.org/arch/x86/tdx.html)
  - **证明链依赖 SGX**：`TDCALL[TDG.MR.REPORT]` 取得 TDREPORT，再交给 **SGX Quoting Enclave** 转成可远程验证的 Quote。（**推断项**：这意味着针对 SGX 证明密钥的攻击如 SGAxe 与 TDX 证明链存在关联风险 —— 本轮未找到直接证据，标注为推断。）
- **AMD SEV / SEV-ES / SEV-SNP** — 四代演进与保护覆盖（引自 [AMD SEV-SNP Primer, arXiv:2608.04039](https://arxiv.org/abs/2608.04039)，2026-08，46 页）：

  | 代次 | 加了什么 | 防住了什么 | 没防住什么 |
  |---|---|---|---|
  | **SME** | 单系统密钥 | 物理内存攻击 | host 软件 |
  | **SEV** | 每 VM 密钥（VEK，按 ASID 选钥） | hypervisor 读 guest 内存 | 寄存器在 VMEXIT 时明文存 VMCB save area |
  | **SEV-ES** | VMSA（加密的 save area）+ `#VC` 异常 + GHCB 协议 | hypervisor 读 guest 寄存器 | replay / corruption / aliasing / remapping |
  | **SEV-SNP** | **RMP**（硬件强制内存完整性）+ **VMPL** | 上述全部 | 见下方"不保护"清单 |

  - **SEV-SNP 核心保证**：guest 读私有页时，要么看到自己最后写入的值，要么收到异常——**绝不会是 stale、被破坏或他人页面的数据**。
  - **RMP**：系统级表，每 **4 KB** 物理页一条 **16 字节**表项（Assigned、ASID、期望 GPA、Validated、Immutable、VMSA、Page_Size、VMPL perms）；翻译后对每次相关访存做内联检查（guest 共享页 C=0 与 hypervisor 读 guest 页不查，加密已保护）。
  - **VMPL** 为 VMPL0–3 四级权限，可实现 guest **内部**的权限分离（如 SVSM）。
  - **加密算法（条件 = CPU 世代）**：Milan 部件用 **AES-128-XEX**；Genoa 及更新用 **AES-256-XTS**，由 launch policy bit 22 `MEM_AES_256_XTS` 约束。
  - **SEV 系列官方列为 out of scope 的威胁**：可用性/资源控制（hypervisor 可随时终止或饿死 guest）、**侧信道与时序泄露**、guest 自身软件漏洞、侵入式物理攻击（总线插桩、DRAM 探测、电压毛刺、开盖）、**信任锚（AMD 制造 / ASP 固件 / 密钥分发）被攻破**。旧版 SEV/SEV-ES **完全没有完整性保护**。
  - **实际攻破案例**：**SEVered**（EuroSec'18，恶意 hypervisor 仅靠 guest 内的远程服务如 web/SSH，即可可靠、高效地明文提取 SEV 加密 VM 的**全部内存**，无需物理访问、无需共谋 VM，高负载下依然有效；原文**未标注 CVE**）[arXiv:1805.09604](https://arxiv.org/abs/1805.09604)；**CVE-2020-12966**（AMD 公告 SB-1013，标题即 "AMD SEV Information Disclosure"，影响 EPYC 上的 SEV-ES 与 SEV-SNP；**具体触发机制本轮未能证实**，AMD 页面抓取失败）[AMD SB-1013](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-1013.html)、[SUSE Bug 1196073](https://bugzilla.suse.com/show_bug.cgi?id=1196073)；**CacheWarp**（CVE-2023-20592，USENIX Sec'24，**架构性软件故障注入**——利用 `INVD` 让 guest 已修改的 cache line 在写回前被作废，使 VM 以 stale 数据继续执行，影响 SEV-ES 与 SEV-SNP；三个演示案例：恢复 Intel IPP RSA 私钥、免认证登录 OpenSSH、经 sudo 提权 root；根因在硬件，缓解困难）[cachewarpattack.com](https://cachewarpattack.com/)；**BadRAM**（IEEE S&P'25，**短暂物理接触内存条的 SPD 芯片**即可在 DDR4/DDR5 制造物理地址别名，绕过 CPU 访问控制，破坏 SEV-SNP 完整性并最终攻破其 **attestation**；对 classic SGX 可做写模式泄露；**Scalable SGX 与 TDX 具备专用别名检测，目前不受该攻击影响**）[badram.eu](https://badram.eu)
  - ⚠️ **术语纠错（重要）**：**CVE-2020-12966 ≠ SEVered**。SEVered 是 EuroSec'18 的设计/架构类攻击，**没有 CVE 编号**；CVE-2020-12966 是 AMD SB-1013 的信息泄露问题。这两者常被混为一谈。
- **Arm CCA / RME** — Arm 官方原文："Arm Confidential Compute (CCA) is a security feature in the **Armv9-A** Architecture that supports isolated environments called **Realms** which are designed to protect sensitive data and code from unauthorized access or modification."（[Arm CCA](https://www.arm.com/architecture/security-features/arm-confidential-compute-architecture)，2026-09-16）
  - **四态**：CCA 把 TrustZone 的两态扩展为 **Normal / Secure / Realm / Root**，由运行在 Root world 的 secure monitor 管理切换。四态编码（官方参考设计）：`SCR_EL3.{NSE,NS}` = `0b00` Secure / `0b01` Non-secure / `0b11` **Realm**；无 Root 编码（处于 EL3 时当前状态恒为 Root）；**Realm 仅定义 EL1/EL0**。[Arm Neoverse RD 文档](https://neoverse-reference-design.docs.arm.com/en/rd-infra-2024.09.30/_sources/shared/rme.rst.txt)
  - **Realm 的关键性质**：由 Normal world 的 Host **动态分配**，且 Realm 及其执行平台的初始状态**可被证明（attestation）**——因此**无需继承 non-secure hypervisor 的信任**。隔离保证：Realm 的执行与内存不可被 Secure 或 Non-secure 的 agent 观测或修改。[Arm Learning Paths: CCA](https://learn.arm.com/learning-paths/cross-platform/cca_rme/cca/)
  - **RME** 是实现 CCA 的主要 Armv9-A 特性；内存由 MMU 经 **Granule Protection Table（GPT）** 动态分配给不同 world。GPT 细节（条件 = Arm AEM FVP 裸机示例，`rme_support_level=2`）：粒度 **16 KB** granule，L0 表由 1 GB 的 L0GPTSZ 划分（示例 PPS=4 GB），基址在 `GPTBR_EL3`、控制位在 `GPCCR_EL3`，由 EL3/Root 配置且**对 Realm 内代码不可见**；把某 granule 改为仅 Secure 后，Realm 的 STR 会触发内存异常。[Arm Learning Paths: 裸机示例](https://learn.arm.com/learning-paths/cross-platform/cca_rme/bare-metal/)
  - **软件栈**：TF-A、**TF-RMM**（Realm Management Monitor）、Linux 内核与 KVM（`lkvm --realm`）；支持 RME 的平台还需 CCA HES 模块（由 RSE 提供）作为 CCA 信任链根；Realm 支持 RSI 与远程证明。正式规范入口为 RME 架构 DEN0125、CCA 安全模型 DEN0096（developer.arm.com 本轮未能直接抓取）。
- **ARM TrustZone** — 更早、更广泛：只有 **Secure world / Normal world** 两态，各跑自己的 OS；**只支持单一 secure world 与固定 secure OS，不适合多个互不信任负载**；通用虚拟化（S-EL2）直到 **Armv8.4-A** 才加入；TrustZone 本身**只提供硬件隔离，不含软件栈**。[SoK arXiv:2512.22090](https://ar5iv.labs.arxiv.org/html/2512.22090)
  - **OP-TEE** 是常用的开源 TEE runtime：实现 GlobalPlatform TEE Internal Core API v1.1.x（面向 TA）与 TEE Client API v1.0（通信）；normal world 的 Linux 称 REE（不可信），secure 侧特权层运行在 Armv7-A PL-1 / Armv8-A EL-1。[OP-TEE 文档](https://optee.readthedocs.io/en/3.8.0/general/about.html)
- **Apple Secure Enclave** — **A7 及更新芯片中的协处理器**，拥有独立 secure boot，其独立软件由 Apple 验证并签名；即使内核被攻破仍可独立工作。每颗 SE 在制造时烧录唯一 **UID**，系统其他部分无法访问、Apple 也不知晓；启动时生成与 UID 绑定的临时密钥，用于加密 SE 在设备内存中的空间。保护对象是**数据保护密钥与生物特征匹配**（Touch ID 指纹数据存在 SE 内并在其中完成匹配，不外发、不备份到 iCloud/iTunes）。**开发者只能通过受限接口把密钥"托付"给 SE**（`SecKey` + Secure Enclave 保护），**不是把任意代码搬进去执行**。
  [Apple Platform Security: The Secure Enclave](https://support.apple.com/guide/security/secure-enclave-sec59b0b31ff/web)、[Apple Developer: Protecting keys with the Secure Enclave](https://developer.apple.com/documentation/Security/protecting-keys-with-the-secure-enclave)（2026-09-16）
  - **定位判断**：它是服务密钥管理与生物识别等**特定功能**的专用安全协处理器，**不是通用计算沙箱** —— 未找到可运行任意第三方负载的公开通用接口。（"非通用沙箱"是按 Apple 文档能力范围作出的判断）

> **本档的位置**：① 与 ② 的区别不是"更强/更弱"，而是**威胁模型不同**。② 假设宿主可信、防的是 guest 逃逸；① 假设宿主不可信、防的是宿主偷看。做 Agent 沙箱时选 ① 通常是**合规驱动**，而非安全驱动。

---

### ② 硬件辅助虚拟机级（Hardware-assisted VM / MicroVM）

| 项 | 内容 |
|---|---|
| **隔离机制** | 硬件虚拟化（Intel VT-x + EPT / AMD-V + NPT / ARM EL2 + Stage-2）→ 内核 KVM → 用户态 VMM → **独立的 guest 内核**。guest 与宿主**不共享内核、不共享页表** |
| **代表产品** | **通用 VMM**：KVM+QEMU、VMware ESXi、Microsoft Hyper-V、VirtualBox；**Cloud/MicroVM（Rust 系）**：Firecracker、Cloud Hypervisor、crosvm、StratoVirt（openEuler）、CubeHypervisor（CubeSandbox 自研，Cloud Hypervisor fork）；**安全容器运行时**：Kata Containers、runV（本仓库笔记） |
| **启动延迟量级** | **100 ms 级及以下**（详见下表数字） |
| **内存与性能开销** | VMM 自身 **单 MB ~ 5 MB 级**；CPU 纯计算接近裸机；syscall 密集负载取决于 virtio/vsock 路径。**注意：guest 内核自身的内存不计入"VMM 开销"，但计入用户配置的 VM 内存** |
| **已知逃逸风险** | 逃逸必须突破**设备模型 → VMM → KVM → CPU**四层。历史 CVE 主要落在**设备模拟**（虚拟网卡、虚拟软驱、SLiRP 等）与**VMM 内存管理**上，而非 CPU 虚拟化本身。**具体 CVE 编号与版本本轮未逐个核实 —— 未能证实** |
| **能否跑异构内核/OS** | **能**。可跑任意 guest 内核、任意 Linux 发行版、Windows、unikernel。这是本档相对 ③④ 的核心优势 |
| **可观测性与运维复杂度** | 高：又多一层 guest 内核要管，镜像、guest agent、vsock、网络/存储后端都要各自运维。但**与容器生态可通过 shim v2 对接**（Kata、CubeShim），运维复杂度可被容器编排吸收 |
| **适用场景** | 多租户不可信代码执行（FaaS / Serverless / Agent 沙箱）、需要独立内核的合规场景、密集合规审计 |

**有出处的启动与开销数字（必附测试条件）：**

| 产品 | 声称值 | 测试条件（原文口径） | 来源 |
|---|---|---|---|
| **Firecracker** | VMM 启动（至 API socket 可用）`8 CPU ms`；**墙钟时间标准差很大，跨度 6 ms ~ 60 ms，典型约 12 ms** | M5D.metal（关闭超线程）、M6G.metal | [SPECIFICATION.md](https://github.com/firecracker-microvm/firecracker/blob/main/SPECIFICATION.md) |
| **Firecracker** | `InstanceStart` → guest `/sbin/init` 起 `≤ 125 ms` | 1 vCPU / 128 MiB、Firecracker 定制内核、**关闭串口、最小内核与 rootfs** | 同上 |
| **Firecracker** | VMM 线程内存开销 `≤ 5 MiB` | 1 vCPU / 128 MiB；依赖工作负载（多 vsock 连接会超 5 MiB）与配置（不含 MMDS 数据） | 同上 |
| **Firecracker** | 稳态变更率 `5 microVMs / 宿主核 / 秒`（36 物理核约 180 个/秒） | 最小 Linux 内核 + 单核 + 128 MiB RAM | [design.md](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md) |
| **Firecracker** | 纯计算 guest CPU 性能 `> 95%` 裸机；网络 `14.5 Gbps` 用 `≤ 80%` 宿主核；虚拟化层平均增加 `0.06 ms` 延迟 | 需专用宿主核做设备模拟线程；**原文标注 `[integration test pending]`** | SPECIFICATION.md |
| **StratoVirt（openEuler）** | microvm 机型 `50 ms 内启动`；内存占用 `< 4 MB`；运行时**系统调用数 `< 46`** | 官方特性页表述，未附硬件条件 | [openEuler StratoVirt 介绍](https://docs.openeuler.openatom.cn/zh/docs/21.09/docs/StratoVirt/StratoVirt%E4%BB%8B%E7%BB%8D.html) |
| **CubeSandbox（腾讯云）** — 宣称值 | README 徽章/表格：冷启动 `< 60 ms`、每实例内存开销 `< 5 MB`、单节点数千实例 | **未附测试条件** | [CubeSandbox README](https://github.com/TencentCloud/CubeSandbox) |
| **CubeSandbox — 官方 benchmark 实测（本次调研唯一条件完整、可复现的一组）** | **串行创建 avg 47.8 ms**（min 43.5 / p95 57.4）；**50 并发 avg 276.1 ms、p95 508 ms、吞吐 147.6/s**（20 并发吞吐 180.9/s 为甜点）；**每 VM 摊销内存 ~21.5 MB @100 实例 → ~25.7 MB @1000 实例**；串行快照 ~50 ms，随脏页近线性增长（**1024 MB 脏页 ~487 ms**），而**从快照创建稳定 60–85 ms**（与脏页无关，on-demand CoW）；rollback ~82 ms；整机 clone ~220 ms（100 沙箱/50 并发时摊销 **5.4 ms**） | **Tencent Cloud BMI5 裸金属；Xeon 8255C 96 逻辑核 / 375 GiB / XFS NVMe；沙箱 2 vCPU + 2 GiB；每场景先预热一轮** | [CubeSandbox Benchmark](https://github.com/TencentCloud/CubeSandbox/blob/master/docs/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md) |

> ⚠️ **CubeSandbox 有三处口径冲突，汇总稿必须并列写出，不能只取好看的一组：**
>
> 1. **内存开销**：README 宣称"每实例内存开销 `< 5 MB`"，官方 benchmark 实测"每 VM 摊销内存 **~21.5 MB @100 实例、~25.7 MB @1000 实例**"，相差约 4~5 倍。合理推测是"`<5 MB` 指 VMM/沙箱管理开销，摊销内存包含 guest 内核与基础运行态"，但**官方未说明口径差异 —— 属未澄清项**。
> 2. **50 并发冷启动**：README 脚注写"50 并发创建 avg **67 ms**、P95 90 ms、P99 137 ms"；而官方 benchmark 报告（沙箱 2 vCPU + 2 GiB、BMI5 裸金属）写"50 并发 avg **276.1 ms**、p95 508 ms"。**两者相差约 4 倍，且都出自官方材料** —— 差异可能来自沙箱规格或预热策略，但官方未说明。引用并发数字时必须写明用的是哪一份材料。
> 3. **单并发冷启动**：README 脚注 `60 ms` vs benchmark 报告串行 **avg 47.8 ms**（min 43.5 / p95 57.4）—— 这一组基本自洽（都在"数十毫秒"量级）。
>
> ⚠️ **"Cloud Hypervisor fork"的措辞需要收紧**：本仓库笔记 `docs/rust-kunpeng/cubesandbox-rust-analysis.md` 记载 CubeHypervisor 为"Cloud Hypervisor fork（v28.0.0）"；而官方 README 的表述是"CubeHypervisor 基于 **RustVMM**，CubeShim 实现 containerd Shim v2"，并在致谢中列出 Cloud Hypervisor / Kata Containers / virtiofsd 等，说明"部分组件为适配运行模型做了定制修改"，**并未自称是 Cloud Hypervisor 分支**。汇总稿建议写成"基于 RustVMM 生态、深度参考并定制了 Cloud Hypervisor / Kata 组件（本仓库笔记将其记为 Cloud Hypervisor v28 fork）"。同理，AgentENV 的"Kimi K3"关联应以官方博客为准（清华 MADSys + Moonshot AI 联合开源，用于 Kimi K3 的 agentic RL 训练）。
| **AgentENV** | boot/resume `< 50 ms`、pause `< 100 ms`、fork `≤ 16` 子沙箱 | **项目自述值**，本仓库笔记转述，本轮未独立复测（`07` 号稿已标记为"项目宣称值"） | 本仓库笔记 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` |
| **Kata Containers** | 无官方统一数字 | Kata 是**运行时框架**，启动时间取决于所选 hypervisor（QEMU / Cloud Hypervisor / Firecracker / StratoVirt），**不存在单一数字** —— 任何"Kata 启动 X ms"的说法都必须先问清 hypervisor 与配置 | [Kata Architecture](https://github.com/kata-containers/kata-containers/blob/main/docs/design/architecture/README.md) |

**Kata Containers 的架构要点（用于澄清常见误解）：** Kata 是"**每 Pod 一个 VM**"的安全容器运行时，**VM 内部仍然用 cgroups + namespaces 再隔离一层 workload**（原文：the container environment created by the agent is equivalent to a container environment created by the `runc` OCI runtime；Linux cgroups and namespaces are created **inside the VM** by the guest kernel）。也就是说 Kata **不是"没有容器"**，而是"**容器套在 VM 里**"——隔离强度来自外层 VM，运维接口来自内层 OCI。
来源：[Kata Containers Architecture](https://github.com/kata-containers/kata-containers/blob/main/docs/design/architecture/README.md)（访问 2026-09-16）

**Rust 化趋势（引用本仓库结论，不重写）：** 从 Firecracker → Cloud Hypervisor → CubeSandbox，MicroVM 领域正从 C/QEMU 迁向 Rust；Cloud Hypervisor 官方自述"导入 rust-vmm crate，与 Firecracker、crosvm 共享代码与架构"，并强调选择 Rust 的原因正是"语言对内存与线程安全的强关注使其成为实现 VMM 的理想候选"。Cloud Hypervisor 官方明确自己与 Firecracker/crosvm 的定位差异：**面向通用 Cloud Workloads，而非容器/Serverless 或客户端场景**。
来源：[Cloud Hypervisor README](https://github.com/cloud-hypervisor/cloud-hypervisor)（访问 2026-09-16）；本仓库 `docs/rust-kunpeng/rust-application-patterns.md` 结论 2

> **② 档内部还要再分层**：通用 VMM（QEMU/ESXi/Hyper-V/VirtualBox）设备模型全、兼容性最好、攻击面最大；MicroVM（Firecracker/crosvm/StratoVirt）设备模型极简、攻击面最小、但兼容性受限（Firecracker 仅暴露 virtio-net/block/vsock 等少数设备）。**"虚拟化"这个词在 ② 档内部的含义差异极大，不可一概而论。**

---

### ③ 用户态内核 / 安全容器（User-space kernel）

| 项 | 内容 |
|---|---|
| **隔离机制** | 在用户态**重新实现一遍 Linux 系统调用接口**（gVisor 的 Sentry），使沙箱内程序**无法直接向宿主内核发起系统调用**；宿主侧只保留一个极小的系统调用面 |
| **代表产品** | **gVisor**（Sentry + Gofer；平台：systrap / KVM / ptrace）、**Kata Containers**（严格说 Kata 属 ②，但常与 gVisor 并列为"安全容器"）、**runV**（Hyper 项目的 OCI 运行时，基于 microVM）、**E2B**、**AgentENV** |
| **启动延迟量级** | 与容器同量级（略高）。gVisor 官方 startup 数据（n1-standard-4 Broadwell，Debian 9）：`alpine true` runc `1193 ms` vs runsc `1144 ms`；`node` `2558 ms` vs `2442 ms`；`ruby` `2530 ms` vs `2456 ms`。**注意原文明确说明：这些数字里大部分是 Docker 自身开销**（证据是空 `runc` 基准也很高），官方建议用 `runsc do` 模式或直接调 OCI runtime 以避开 |
| **内存与性能开销** | 固定内存开销约 **20 MiB 量级**（见核心结论 6）；syscall 开销**由平台决定**（见核心结论 5）。CPU 纯计算与内存带宽**无额外开销**（官方：gVisor 不做模拟、不干扰 CPU 指令的直接执行；页映射建立后无额外开销） |
| **已知逃逸风险** | 逃逸需先攻破 Sentry（Go 实现）或其平台拦截机制，再攻破宿主内核。gVisor 的公开立场是"即使沙箱自身被攻破，仍有若干防御纵深层（含受限 seccomp 过滤器）" |
| **能否跑异构内核/OS** | **不能**。沙箱内仍是 Linux ABI 兼容层，跑不了非 Linux guest。异构能力弱于 ②、强于 ④ |
| **可观测性与运维复杂度** | 中高：多一层用户态内核意味着**部分 `/proc`、`io_uring`、原始套接字等行为与原生不一致**，需要处理兼容性问题；与容器生态兼容性好（OCI runtime 形态） |
| **适用场景** | 不具备嵌套虚拟化条件的云环境（如部分 ARM/云主机）、需要比容器强但比 microVM 轻的场景、gVisor 兼容范围内的无状态服务 |

**平台差异是 ③ 档的核心工程事实（gVisor 官方 systrap 发布博客）：**

- **ptrace 平台**：通过 `PTRACE_SYSEMU` 把系统调用交回 Sentry，**任何环境都能跑，但很慢**。官方 `getpid` 微基准（GCE n2-standard-4，debian-11-bullseye-v20230306）用来量化这一开销。
- **KVM 平台**：Sentry 同时扮演 guest OS 与 VMM，裸金属虚拟化下性能好，但**嵌套虚拟化下明显变差**（且部分 CPU/云环境不支持嵌套）。
- **systrap 平台**：用 ptrace 初始化 executor 线程 + **非常严格的 seccomp 过滤器** + 自定义 `SIGSYS` 信号处理器 + 与 Sentry 共享内存通信；更进一步把 x86_64 上的 `mov sysno, %eax; syscall` 指令模式（共 7 字节）**热替换为 `jmp *%gs:offset` 跳板**（把系统调用号编码进 offset）。**官方明确：ARM 上当时没有这个优化**。
- **官方结论：systrap 于 2023 年 9 月取代 ptrace 成为默认平台**（这一时间点已过去，当前默认平台应为 systrap —— 本轮未再核实最新默认值）。
  来源：[Releasing Systrap](https://gvisor.dev/blog/2023/04/28/systrap-release/)（访问 2026-09-16）

> **③ 与 ④ 的分界**：④ 的隔离完全依赖宿主内核自身的机制（namespace/cgroup/seccomp/LSM），因此**强度受宿主内核漏洞支配**；③ 用用户态内核把系统调用面收窄，**攻击者要打的是 Go 写的 Sentry 而不是 20 年历史的 Linux 内核**。这是 ③ 档的全部价值所在。

---

### ④ OS 级隔离（共享内核）

| 项 | 内容 |
|---|---|
| **隔离机制** | Linux 内核原语叠加：**namespaces**（mnt/pid/net/ipc/uts/user/cgroup/time）+ **cgroups**（CPU/内存/IO/pid 限额）+ **capabilities**（能力集裁剪）+ **seccomp-bpf**（系统调用过滤）+ **Landlock**（无特权路径访问控制 LSM）+ **AppArmor / SELinux**（强制访问控制 LSM）+ `no_new_privs` |
| **代表产品** | runc / containerd / Docker / Podman（容器）；**bubblewrap**（无特权用户 namespace 沙箱）；**Flatpak**（桌面应用沙箱，基于 bubblewrap）；**systemd** 的沙箱化选项；Firejail；**DSH 自身的沙箱（见后文专节）** |
| **启动延迟量级** | **最低**。本仓库 `07` 号稿给出"容器启动约 `10 ms` 量级、内存开销近零"作为量级描述；注意这一量级指的是**运行时层创建容器的开销**，而 SOSP'17 实测的 `Docker 容器启动约 150 ms` 含完整 Docker 路径 —— **两者口径不同，不可混用** |
| **内存与性能开销** | 近零（共享内核页、共享 page cache、共享二进制）。seccomp 过滤器本身有可测量的开销（gVisor 官方博客引用 LWN《A seccomp overview》指出 seccomp 过滤器开销"not insubstantial"） |
| **已知逃逸风险** | **最高，且已有大量真实案例**：CVE-2019-5736（runc 覆盖宿主二进制 → 宿主 root）、CVE-2022-0492（cgroup v1 `release_agent` 绕过 namespace 隔离提权）、CVE-2024-21626（runc fd 泄漏 → 逃逸）。此外还有能力误配置（`--privileged`）、挂载宿主 docker.sock、`CAP_SYS_ADMIN` 滥用等**配置类逃逸** |
| **能否跑异构内核/OS** | **不能**。共享宿主内核，跑不了别的内核；Windows 容器也只能跑与宿主同版本族的容器 |
| **可观测性与运维复杂度** | **最低**：无 guest、无 hypervisor，`kubectl exec`、`strace`、`/proc` 语义都是原生的 |
| **适用场景** | 可信工作负载的多进程隔离、资源配额、非安全边界性的环境隔离（CI 构建、开发环境）、以及**作为更强隔离方案的"内层"**（Kata 的 guest 内就复用这套） |

**bubblewrap 官方对自身定位的表述（很重要，直接反驳"用了 bwrap 就安全"）：**

> "bubblewrap is a tool for constructing sandbox environments. bubblewrap is **not a complete, ready-made sandbox with a specific security policy**."
> "the level of protection between the sandboxed processes and the host system is **entirely determined by the arguments passed to bubblewrap**."

以及其自述的已知局限（逐条摘自官方 README）：

- 若不通过 seccomp 过滤 `TIOCSTI`，**必须**加 `--new-session`，否则可被用于"沙箱外命令执行"（对应 [CVE-2017-5226](https://github.com/containers/bubblewrap/issues/142)）；
- **任何被挂载进沙箱的东西都可能被用来提权**——例如把 D-Bus socket 绑进沙箱后，可用于通过 systemd 执行命令（官方建议用 `xdg-dbus-proxy` 过滤）；
- 沙箱策略可能**反噬应用自身的沙箱机制**：如果限制了 `seccomp` 系统调用，浏览器就无法给自己的子进程施加 seccomp 约束。
  来源：[bubblewrap README](https://github.com/containers/bubblewrap)（访问 2026-09-16）

**systemd 沙箱化选项（官方 `systemd.exec(5)` 的分类）：**

- **路径类**：`RootDirectory=` / `RootImage=`（+`RootHash=`/`RootVerity=` 做 dm-verity 完整性校验）/ `RootEphemeral=`（在 `/var/lib/systemd/ephemeral-trees/` 下做临时副本，btrfs 快照或 reflink）/ `RootMStack=`（overlayfs 栈式 rootfs）；
- **保护类**：`ProtectSystem=`（true / full / strict）、`ProtectHome=`（yes / read-only / tmpfs）、`ProtectProc=`（`hidepid=` 语义）、`PrivateTmp=`、`PrivateDevices=`、`PrivateUsers=`、`ProtectKernelTunables=`、`ProtectControlGroups=`、`ProtectClock=`；
- **系统调用类**：`SystemCallFilter=`、`SystemCallArchitectures=`、`RestrictAddressFamilies=`、`RestrictNamespaces=`、`RestrictRealtime=`、`RestrictSUIDSGID=`、`MemoryDenyWriteExecute=`、`LockPersonality=`、`NoNewPrivileges=`。

**官方自己写明的两条重要局限（写文档时容易被忽略）：**

1. `ProtectSystem=` / `ReadOnlyPaths=` 这类"把目录设为只读"的选项**不影响程序连接并与其通信的 `AF_UNIX` socket**——因此**不能**用来锁死 IPC 服务访问；
2. 许多沙箱化能力在**用户服务**（per-user service manager）中不可用，因为底层内核功能仅对特权进程开放；`ProtectSystem=` 在"内核未启用文件系统命名空间"或"运行在使命名空间不可用的容器里"时**直接静默失效**（gracefully turned off）。
   来源：[systemd.exec(5)](https://man7.org/linux/man-pages/man5/systemd.exec.5.html)（访问 2026-09-16）

**Landlock 的能力边界（官方内核文档，直接决定 DSH 沙箱的上限）：**

- Landlock 是**可叠加（stackable）的 LSM**，目标是让**任何进程（含无特权进程）安全地自我限制**；一旦某个线程被 landlock，**策略不可移除，只能继续收紧**，并自动继承给所有子进程。
- **ABI 版本决定能力**：ABI 1 起有文件系统规则；`LANDLOCK_ACCESS_FS_REFER` 自 **ABI 2**；`TRUNCATE` 自 **ABI 3**；**TCP 端口规则自 ABI 4**；`IOCTL_DEV` 自 **ABI 5**；`SCOPE_ABSTRACT_UNIX_SOCKET` / `SCOPE_SIGNAL` 自 **ABI 6**；`FS_RESOLVE_UNIX` 自 **ABI 9**；**UDP 端口规则自 ABI 10**；`LANDLOCK_RESTRICT_SELF_NO_NEW_PRIVS` 自 **ABI 11**。
- **最大的能力缺口**：官方明确列出**当前无法通过 Landlock 限制**的文件相关操作，包括 `chdir(2)`、`stat(2)`、`flock(2)`、`chmod(2)`、`chown(2)`、`setxattr(2)`、`utime(2)`、`fcntl(2)`、`access(2)`。也就是说 Landlock **限制的是"打开/创建/删除/重命名/执行"这类路径解析动作，不是元数据操作**。
- **Landlock 与 OverlayFS 的语义不合**：bind mount 的访问权可随挂载传播，但 **OverlayFS 的 upper/lower/merge 三层在 Landlock 眼里是各自独立**的——限制某一层**不会**限制合并后的目录，反之亦然。
  来源：[Landlock: unprivileged access control](https://docs.kernel.org/userspace-api/landlock.html)（内核文档，标注 August 2026；访问 2026-09-16）

---

### ⑤ 应用 / 语言级（Application & language level）

这一档与前四档**正交**：它不提供"整机隔离"，而是在**单个进程/虚拟机内部**划定边界。它通常作为 ①~④ 的**纵深防御内层**存在（浏览器渲染进程既跑在自己的沙箱里，又跑在 OS 沙箱里）。

| 子类 | 隔离机制 | 代表产品 | 关键事实与出处 |
|---|---|---|---|
| **浏览器渲染进程** | 每平台各一套：**Windows** 用 restricted token + 独立 job object + alternate desktop + integrity levels；**Linux** 用 namespaces + Seccomp-BPF；**macOS** 用 Seatbelt | Chrome / Chromium | Chromium 官方 `sandbox/README.md` 原文："Each platform relies on the operating system's process primitive to isolate code into distinct security principals, and platform-specific technologies are used to implement the privilege reduction." 其上还有 `//sandbox/policy` 组件提供具体策略。[来源](https://chromium.googlesource.com/chromium/src/+/main/sandbox/README.md) |
| **WebAssembly** | **线性内存隔离**（指针编译为线性内存偏移、所有访问做边界检查）+ **能力模型**（不能直接发起系统调用，只能通过被显式链接的 import 与外界交互）+ 不可访问的调用栈 | Wasmtime（WASI）、WAMR | Wasmtime 官方：核心 Wasm 规范提供"调用栈不可访问""指针即线性内存偏移且在界内检查""所有控制转移都是已知且类型检查过的目标""与外界的一切交互都通过 import/export""不存在未定义行为"。WASI 文件系统访问遵循**基于能力的安全模型**。[来源](https://docs.wasmtime.dev/security.html) |
| **Java SecurityManager** | 运行期权限检查（字节码栈上检查调用方） | 已**弃用并移除** | 本条**未能证实**具体 JEP 编号与版本（本轮抓取 openjdk.org 失败）。写汇总稿前须核实 JEP 411 与 JEP 486 的确切标题与目标版本 |
| **.NET Code Access Security (CAS)** | 运行期基于证据的权限集 | 已**弃用**；.NET Core 起不再支持 | 本轮**未能证实**官方弃用文档的具体措辞与版本 —— 待补 |
| **iOS / Android 应用沙箱** | 每应用独立 Linux UID + 独立进程 + 独立运行时实例；叠加 SELinux 与 seccomp（Android） | Android / iOS | Android 官方 AOSP《Application Sandbox》文档为权威来源（[source.android.com](https://source.android.com/docs/security/app-sandbox)）；**本轮抓取正文被导航栏截断，未取得可逐字引用的原文 —— 具体机制措辞待核实** |
| **Windows AppContainer / MSIX** | **运行期文件系统与注册表虚拟化** + AppContainer 安全边界（**非虚拟机、无需独立 OS 镜像**） | MSIX / UWP | 微软官方：MSIX"不是虚拟机，也不需要独立 OS 镜像"；写安装目录与系统位置被阻止，用户配置位置重定向到按包隔离的位置；**两档信任级别**：Full trust（中等完整性，仍可直接访问多数系统资源）与 AppContainer（部分信任，进程及子进程**只能访问被显式授予**的资源，UWP 永远跑在 AppContainer 中）[来源](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview) |
| **macOS Seatbelt** | `sandbox-exec` + SBPL 策略（`allow default` / `deny file-write*` + 逐路径 `allow`） | 系统级 CLI | Chromium 与 DSH 均使用。**注意：Apple 已将该 CLI 标记为 deprecated 但仍在每个 macOS 上提供**（DSH 源码注释原文："Apple marks the CLI deprecated but ships it on every macOS; if it ever disappears, this probe is what fails closed"） |

> **本档的定位提醒**：⑤ 档**不能**用来替代 ②/③/④。它在同一地址空间/同一内核内工作，**无法防御宿主内核漏洞，也无法阻止侧信道**（Wasmtime 官方单列 Spectre 缓解章节，说明这一层需要额外付出）。它的正确用法是**纵深防御的内层**与**进程内的能力最小化**。

---

### ⑥ 网络 / 数据沙箱（Network & data egress）

这是**正交的第二个维度**：即使前面 ①~⑤ 都做对了，出口仍然可以泄露数据。分为两个子问题：

**(a) 受限网络出口（egress control）**

- **内核强制（最强）**：独立 **network namespace**（bubblewrap 的 `--unshare-net`：沙箱只看到 loopback）+ CNI 策略 + 主机防火墙。DSH 的 bwrap profile **没有**使用这一项（详见后文）。
- **内核强制（端口粒度）**：**Landlock ABI 4+ 的 TCP 规则、ABI 10+ 的 UDP 规则**可以在**端口粒度**上限制 `bind`/`connect`。注意：Landlock 网络规则的粒度是**端口号，不是 IP 或域名**——因此它挡不住"允许访问 443 就等于允许访问任意 HTTPS 站点"这一根本问题。
  来源：[Landlock 内核文档](https://docs.kernel.org/userspace-api/landlock.html)（访问 2026-09-16）
- **代理强制（可绕过）**：进程级 HTTP(S) 代理环境变量 + 出口网关。**关键弱点：只覆盖走标准 transport 的调用**，自带网络栈的 SDK、原生子进程、原始套接字都可能绕过。
- **官方提醒**：Firecracker 官方明确写"**Firecracker 不执行任何网络流量过滤**。来自 guest 的所有出口流量都被视为不可信，应在宿主层过滤。"（[design.md](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)）→ **microVM 解决的是"逃逸"，不解决"外联"。**这是 Agent 沙箱设计中最容易漏掉的一环。

**(b) 只读文件系统 / 叠加层（data integrity & rollback）**

- **只读 bind mount**：最通用（bwrap `--ro-bind`、Landlock `--ro` 授权、systemd `ProtectSystem=`）。
- **OverlayFS / overlay 层**：lower 层只读、upper 层可写、merge 层呈现。**注意 Landlock 对 OverlayFS 的语义不合**（见 ④ 档末），两者叠加时不能想当然。
- **reflink 快照**：XFS `FICLONE` 元数据级 O(1) 克隆（CubeSandbox 的 CubeCoW 走这条路线，见本仓库 `docs/rust-kunpeng/cubesandbox-rust-analysis.md` 第四节）。
- **dm-verity**：systemd `RootHash=` / `RootVerity=` 对镜像做完整性校验；MSIX 用签名 + 运行期篡改检测（检测到篡改则阻止启动并触发修复）。
- **临时/丢弃式根**：systemd `RootEphemeral=`（btrfs 快照或 reflink 的临时副本，服务停止即清理）——这是"**回滚**"诉求在 OS 层的现成实现。

---

## 沙箱 vs 虚拟化 对照

### 主对照表

| 维度 | ① 机密计算 | ② 通用 VM（QEMU/ESXi/Hyper-V） | ② MicroVM（Firecracker/crosvm/CH/StratoVirt） | ③ 用户态内核（gVisor） | ④ 容器（namespaces） | ⑤ WASM / 应用级 |
|---|---|---|---|---|---|---|
| **隔离边界** | CPU TCB（+固件） | guest 内核 + VMM + KVM + CPU | 同左，但设备模型极简 | 用户态 Sentry 拦截 syscall | 宿主内核的 LSM/namespace | 语言运行时 / 进程内 |
| **共享内核？** | 否（可连 hypervisor 都不信任） | 否 | 否 | **是**（共享宿主内核，但程序不能直接调用） | **是** | **是** |
| **共享页表？** | 否（EPT/NPT 二级翻译 + 内存加密） | 否（EPT/NPT） | 否（EPT/NPT） | 部分（Sentry 管理 guest 页表，宿主页表由内核管理） | **是**（同一地址空间体系） | 线性内存由运行时软件检查 |
| **单实例内存开销** | 无公开可比数字 —— **未能证实** | 数十 MB ~ 数百 MB（VMM + 设备模型） | **≤ 5 MiB（Firecracker 官方，1vCPU/128MiB）**；**< 4 MB（StratoVirt 官方）**；**< 5 MB（CubeSandbox 官方，规格≤32GB）** | **约 20 MiB 量级**（官方密度实验：空容器 4.09 MB → 23.70 MB） | **近零**（共享内核与 page cache） | 每实例数 MB 级线性内存 —— 具体数字**未能证实** |
| **冷启动延迟** | 未取得可比口径数字 —— **未能证实** | 秒级（含固件/UEFI 引导） | **≤ 125 ms（Firecracker，含 guest init）**；**50 ms 内（StratoVirt microvm）**；**60 ms 单并发（CubeSandbox 裸金属）** | 与容器同量级（官方 startup 数据被 Docker 开销主导） | **约 150 ms（SOSP'17 实测 Docker）/ 运行时层约 10 ms 量级** | 亚毫秒 ~ 数十毫秒（Wasmtime 有 fast instantiation 路径）—— 具体数字**未能证实** |
| **稳态性能开销** | 内存带宽/延迟有开销，具体百分比 **未能证实** | I/O 与 syscall 密集负载有开销；纯计算接近裸机 | **纯计算 > 95% 裸机（Firecracker 官方，但标注 integration test pending）**；网络 `14.5 Gbps`@`≤80%`核；虚拟化层平均 +`0.06 ms` | **强依赖平台**：syscall `1939 ns`(runc) / `38219 ns`(ptrace) / `763 ns`(KVM)；CPU 与内存带宽**无额外开销** | 近原生 | 计算有 JIT/解释开销；边界检查有开销 |
| **单机密度** | 取决于加密引擎与内存带宽 —— **未能证实** | 低（每 VM 一个内核 + VMM 进程） | **高**：CubeSandbox 官方称"每节点数千个"；Firecracker 官方称"仅在硬件资源可用性上受限" | 中（每实例多约 20 MiB） | **最高**（仅受 cgroup 限额约束） | 极高（同一进程内多实例） |
| **逃逸难度** | 不与"逃逸"同一威胁模型；风险在侧信道与可用性 | 高（4 层边界） | 高（4 层边界，设备模型更小→攻击面更小） | 中高（需先破 Sentry） | **最低**（已有 CVE-2019-5736 / CVE-2022-0492 / CVE-2024-21626） | 中（需破验证器或宿主函数） |
| **历史 CVE 类别** | 侧信道、微架构缺陷、微码 —— 编号本轮未核实 | **设备模拟**（虚拟网卡/软驱/SLiRP）、VMM 内存管理 | 设备模拟（面更小）、virtio 后端 | Sentry 实现漏洞、平台拦截机制漏洞、兼容性缺口 | **运行时**（runc fd/二进制覆盖）、**内核**（cgroup/namespace）、**配置**（privileged/挂载 docker.sock） | 验证器/JIT 缺陷、宿主函数漏洞、WASI 能力配置错误 |
| **能否跑异构内核/OS** | **能**（可跑完整 guest OS） | **能**（Linux/Windows/任意发行版） | **能**（但设备模型受限，需兼容性适配） | **不能**（Linux ABI 兼容层） | **不能**（共享宿主内核） | **不能**（需编译到 wasm） |
| **可观测性 / 运维复杂度** | 最高（客机不可见 → 调试极难） | 高（guest 内核、镜像、agent、后端网络存储） | 中高（同左，但可经 shim v2 接入 containerd） | 中（部分 `/proc`、io_uring、原始套接字行为不一致） | **最低**（原生 `exec`/`strace`/`/proc`） | 低（进程内），但缺乏标准运维接口 |

### 对照表的四条读法

1. **"快"与"安全"不是同一条轴上的取舍，而是"共享内核 vs 不共享内核"的跳跃。** 一旦跨过"独立 guest 内核"这条线，代价从"近零"跳到"几 MB + 几十毫秒"；再往上（①）增加的是**威胁模型的改变**，不是隔离强度的线性提升。
2. **② 档内部的差异（QEMU vs Firecracker）比 ② 与 ③ 的差异更值得关注。** Firecracker 的价值不在"它是 VM"，而在"它把 VM 的设备模型和攻击面砍到只剩 3 个 virtio 设备"，从而把 VMM 启动压到 `8 CPU ms`、内存压到 `≤ 5 MiB`。
3. **③ 档的性能不能用一个数字描述。** 同一份 gVisor 官方数据里 ptrace 平台是 runc 的 ~19.7 倍，KVM 平台反而更快。**引用 gVisor 性能时必须同时写明平台**，否则结论无效。
4. **密度数字必须看测试条件。** Firecracker 的 `5 microVMs/核/秒` 是"最小内核 + 单核 + 128 MiB"的理想配置；CubeSandbox 的 `< 5 MB` 是"规格 ≤ 32 GB"；gVisor 的密度实验是"50 个实例（redis 为 5 个）"取平均。**脱离条件的密度数字没有意义。**

---

## AI Agent 沙箱

### 为什么 LLM / Agent 场景需要沙箱

| 诉求 | 具体含义 | 对应到沙箱档位 |
|---|---|---|
| **不可信代码执行** | LLM 生成的代码/命令不可审计、不可预测；可能被 prompt injection 诱导 | 需要**至少 ② 档**（独立内核）或 **③ 档**（用户态内核），而非 ④ |
| **多租户** | 同一台机器上跑不同用户/不同会话的 Agent | 需要 CS 级别的隔离 + 配额；② 档的独立内核是关键 |
| **资源限额** | CPU/内存/进程数/PID/磁盘/文件描述符上限，防止"一个 Agent 打爆一台机器" | ④ 的 cgroups 也能做，且更细；② 需要 guest 内叠加 cgroup（Kata 就是这样做的） |
| **网络出口控制** | 防止数据外泄、防止 SSRF 打内网、限制可访问的 API/包源 | **⑥ 维度，任何档位都不自动提供**。microVM 官方明确不做流量过滤；容器需要 CNI 策略；DSH 用进程级代理 |
| **快照 / 回滚** | Agent 会破坏环境，需要"回到上一步"；RL 训练需要从同一状态 fork 多条轨迹 | ② 档专属优势：内存快照（AgentENV）或磁盘 reflink（CubeSandbox）。④ 档的 overlay 只能回滚文件系统，回滚不了进程状态 |
| **pause-resume** | 会话闲置时挂起省资源，再次使用时毫秒级恢复；跨节点迁移 | ② 档专属：CubeSandbox 支持跨节点 pause/resume（S3 后端）；④ 档进程无法真正"暂停到磁盘再恢复" |

> **一句话**：Agent 沙箱的核心诉求是"**不可信代码 + 有用状态**"这一对矛盾——既要把代码关起来，又要让它有文件系统、有进程、能装包、能跑起来。**只有 ② 档能同时满足**，这就是为什么所有主流 Agent 沙箱产品最终都落在 microVM 上。

### 产品形态对比

> **可信度分级（本轮）**：**可复现实测只有一家** —— CubeSandbox 官方 benchmark（条件完整）。**有明确条件的厂商数字**：microsandbox（"M1 机器上的 guest 启动"）、Fly.io（区分"已有 Machine 启动"与"首次创建"）、Cloudflare（给出区间而非单点）。**其余全部是无条件的厂商声明**，引用时须标注"厂商自述、未附测试条件"。

| 产品 | 隔离档位 | 关键形态 | 有出处的数字 |
|---|---|---|---|
| **E2B** | ②（**Firecracker microVM**，每沙箱独享内核） | 运行时 **Apache-2.0 开源**，支持自托管/BYOC（E2B Embed）。**本仓库 `sandbox_research.md` 把它列为需要向合作方确认定位的核心对象**（"纯 SaaS 还是也有客户端本地运行"仍是待确认问题）。API：JS/Python SDK（`create/connect/pause/kill/setTimeout`、`commands.run`、files、`getHost(port)`、metrics）+ CLI + Template 构建体系 | 官方文档对比表称 **~150 ms**（"from a snapshot or custom template"），**未给硬件/负载条件**；模板页称运行态快照可 **~80 ms** 后加载（进程已运行），同样无条件。**生命周期**：默认 timeout **5 分钟**；连续运行上限 Hobby 1h / Pro 24h / Enterprise 自定义；**paused 沙箱无 TTL、无限期保留、不计费、不占并发额度**。**快照**：`pause()` 同时保存文件系统 + 内存（进程与变量存活），`keepMemory:false` 退化为仅 FS 冷启；`createSnapshot` 命名快照；`fork` 从运行态直接派生（官网称最多 100）。[docs](https://docs.e2b.dev/)（2026-09-16） |
| **AgentENV（AENV）** | ②（**Firecracker microVM** + E2B 兼容 HTTP API） | **清华 MADSys Lab 与 Moonshot AI 联合开源**，已用于 **Kimi K3** 的 agentic RL 训练；称典型负载下 agent 环境成本降低 **88.6%–96.8%**。机制：overlaybd + ublk 分层 CoW 块设备、内存与磁盘快照、运行态 fork、内存 balloon 回收、快照落 S3 兼容存储/共享分布式 FS。依赖 **Linux kernel 6.8+** 与 `/dev/kvm`。[官方博客](https://kvcache.ai/blog/agentenv-open-sourced/)｜[GitHub](https://github.com/kvcache-ai/AgentENV) | 快照启动/恢复 **< 50 ms**、pause **< 100 ms**、增量内存+文件系统快照 **< 100 ms**（重磁盘修改下）、生产内存 overcommit **9.6×**、镜像规模 **150 万** —— **官方文档/README 自述，未附测试硬件**。`07` 号稿已标记为"项目宣称值"，本轮未独立复测。fork 上限 `≤ 16` 子沙箱见本仓库笔记（**产品硬限制还是实现约束仍未证实**） |
| **CubeSandbox** | ②（**RustVMM + KVM**；CubeHypervisor + CubeShim） | 腾讯云开源（Apache-2.0）；containerd Shim v2 集成；**兼容 E2B SDK**；CubeCoW 基于 **xfs-reflink（内核 `FICLONE` ioctl）** 做 O(1) 快照/克隆；v0.7 起支持跨节点 pause/resume 与从快照创建（S3 后端） | 见 ② 档表格 —— **宣称值与 benchmark 实测并存且口径冲突**，引用时必须并列 |
| **Daytona** | **默认 ④ 容器**（独立 namespace + 资源限制），另有 Linux VM / Windows / GPU / macOS 沙箱 | 面向 AI 生成代码的弹性沙箱基础设施，sandbox = "full composable computer"。⚠️ **官方文档中"dedicated kernel"的营销语与"默认容器"并存，但容器沙箱不提供独立内核** —— 这是产品对比中极易被误读的一处 | "**under 90ms from code to execution**"（文档原话，**无测试条件**）。**重要限制**：容器沙箱**不支持 pause/resume 与 fork**（stop 仅保留文件系统，cold snapshot 仅存 FS）；**只有 VM 沙箱**才支持 pause/resume 与内存持久化。[sandboxes](https://www.daytona.io/docs/en/sandboxes.md)（2026-09-16） |
| **microsandbox** | ②（**libkrun microVM**） | local-first microVM 运行时；每沙箱独享 Linux 内核（由 **libkrunfw** 构建），经 **libkrun VMM** 调用 KVM（Linux）/ Apple Hypervisor.framework（macOS），Windows 走 WHP；**宿主进程以普通用户运行、无需 root**，仅需 `/dev/kvm` 或 hypervisor entitlement。SDK：TypeScript / Rust / Python / Go / Ruby + `msb` CLI + MCP server。**自托管形态最轻：SDK 直接把 microVM 作为子进程拉起，无 daemon、无服务端依赖**；支持 `msb branch`、snapshot create/restore、detached 长驻 | "平均 guest boot **< 100 ms**"，**脚注给出条件："M1 机器上的 guest 启动"** —— 这是少数带条件的厂商数字。**每实例内存开销官方未给 → 未能证实**。[isolation](https://docs.microsandbox.dev/security/isolation)｜[GitHub](https://github.com/superradcompany/microsandbox)（2026-09-16） |
| **Kata Containers** | ②（多 hypervisor） | 每 Pod 一 VM，VM 内再跑 OCI 容器；shim v2；agent 用 Rust，vsock + ttRPC 通信；支持 Rego 策略（`kata-runtime policy set policy.rego`） | **无单一数字**（取决于所选 hypervisor 与配置） |
| **Modal** | ③（**gVisor**） | 容器 + gVisor 虚拟化 | 官方 cold start 文档称 "**Containers boot in about one second**"（**无硬件条件**，含排队与初始化差异）[security](https://modal.com/docs/guide/security)｜[cold-start](https://modal.com/docs/guide/cold-start) |
| **Fly.io Machines** | ②（**Firecracker**） | — | **已有 Machine 启动通常"远低于 1 秒"**；**首次创建（拉镜像 + 建 rootfs）"低双位数秒"级**；跨区 API 往返 20–50 ms（同区）与 ~300 ms（悉尼↔弗吉尼亚）示例。[fly blog](https://fly.io/blog/fly-machines/)（2026-09-16） |
| **Cloudflare Sandbox / Containers** | ②（每实例独立 VM） | Containers + Durable Objects 管生命周期 | 冷启动"**often in the 1–3 second range**"，依 entrypoint 而定，并预热/预取镜像。[架构](https://developers.cloudflare.com/sandbox/concepts/architecture/)（2026-09-16） |
| **Northflank** | ②+③（**microVM-backed container** + 用户态内核隔离） | — | 官方称 **boot < 1 秒**（无条件）[文档](https://northflank.com/docs/v1/application/sandboxes/sandboxes-on-northflank.md) |
| **Vercel Sandbox** | ②（**Linux microVM**，自有 Hive 平台，**Firecracker** 驱动） | — | 官方只提 "sub-second starts" 目标与客户证言，**未发布自测冷启动数字 → 未能证实**[GA 博客](https://vercel.com/blog/vercel-sandbox-is-now-generally-available) |
| **Morph** | ②（快照/分支型） | Infinibranch 快照/分支/恢复 | **< 250 ms**（官方声明，无测试条件）[docs](https://cloud.morph.so/docs/developers) |
| **Runloop** | ②（隔离 VM） | Devbox 按需创建 | 官方原文"startup to running your first command takes **a few seconds**"。[文档](https://docs.runloop.ai/docs/devboxes/overview) |
| **Blaxel** | ②（轻量沙箱 VM） | standby → 恢复，空闲数秒后 scale-to-zero 且保留内存态 | 从 standby 恢复 **< 25 ms**（官方声明，无测试条件）[文档](https://github.com/blaxel-ai/docs/blob/main/Sandboxes/Overview.mdx) |
| **CodeSandbox / Together Code Sandbox** | ②（microVM） | — | 从模板克隆、从快照 resume VM "**under three seconds**"。[Together 文档](https://docs.together.ai/docs/together-code-sandbox) |

### 同类 Agent 编码工具的沙箱选择（与 DSH 直接可比）

这一组最有参考价值——它们是**同类产品在同一问题上的不同取舍**，且都属 ④ 档：

| 产品 | macOS | Linux | 网络出口 | 备注 |
|---|---|---|---|---|
| **Claude Code 内置 Bash sandbox** | 系统 **Seatbelt**（零安装） | **bubblewrap**（文件系统隔离）+ **socat**（把流量中继到沙箱代理，实现**按域名的网络隔离**）+ 可选 **seccomp filter** 阻断 Unix domain socket | **有**（代理 + 域名白名单） | Ubuntu 24.04+ 需额外 AppArmor profile 放行非特权 user namespace。[官方文档](https://code.claude.com/docs/en/sandboxing)（2026-09-16） |
| **Anthropic Sandbox Runtime（`srt`，研究预览）** | `sandbox-exec`/Seatbelt 动态生成 profile | **bubblewrap** | **有**（代理白名单） | 可沙箱化任意进程与 MCP server。[GitHub](https://github.com/anthropic-experimental/sandbox-runtime) |
| **OpenAI Codex CLI** | **Seatbelt**（`codex-rs/sandboxing/src/seatbelt.rs` + `seatbelt_*.sbpl`） | **`no_new_privs` + seccomp**，**文件系统隔离现由 bubblewrap 执行；Landlock 仅保留为 legacy/backup** | 未核实 | ⚠️ **纠错**："Codex = macOS Seatbelt + Linux Landlock+seccomp" 的说法**已部分过时**——现行 Linux 实现是 bubblewrap + seccomp/no_new_privs，Landlock 为历史路径。官方说明页 `developers.openai.com/codex/security` 本次抓取被 **403** 拦截，**官方原文未能证实**，上述结论来自源码。[seatbelt.rs](https://github.com/openai/codex/blob/main/codex-rs/sandboxing/src/seatbelt.rs)｜[landlock.rs 注释](https://github.com/openai/codex/blob/main/codex-rs/linux-sandbox/src/landlock.rs) |

**三条对 DSH 有直接参考价值的观察：**

1. **同类产品几乎都收敛到同一套机制组合**：macOS Seatbelt + Linux bubblewrap/`no_new_privs`/seccomp。**DSH 的 runner 链与行业做法一致**，且 DSH 额外提供了 Landlock 后备与 Windows ACL runner —— **在平台覆盖面上 DSH 比同类更广**（Codex 与 Claude Code 的 Windows 路径未在本次核实范围内）。
2. **DSH 与 Claude Code 的差距精确地落在"网络出口"上。** Claude Code 用 **socat + 代理 + 域名白名单**把出口控制落到了沙箱层；DSH 的 bwrap profile **无 `--unshare-net`**，出口控制在进程级 HTTP 代理，且**自带 transport 的调用可绕过**。**这是 DSH 沙箱最值得补的一块**，且补法有现成参考。
3. **Anthropic 把沙箱能力抽成了独立可复用组件（`srt`）**，可沙箱化任意进程与 MCP server。DSH 的 `ctx.sandbox` 服务契约 + 可替换 backend 是同一种"能力与消费者解耦"的设计，**方向一致**。

### CubeSandbox 与 AgentENV 的架构差异（直接引用本仓库笔记，不重写）

两者都是 rust-vmm + KVM MicroVM、都兼容 E2B API、都以 Rust 为控制面语言、都把 snapshot/restore 当核心原语。**本质差异在 CoW 路径与集群形态**：AgentENV 走"**内存级 CoW（可 fork）+ 分布式**"，服务大规模 RL 训练；CubeSandbox 走"**磁盘级 CoW（XFS reflink）+ containerd 单节点集成**"，服务容器化安全沙箱。
来源：本仓库笔记 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`、`07-existing-notes-map.md` 第四节

### DeepSeek Harness (DSH) 自身的沙箱实现

以下内容来自 DSH 发布包源码（`@deepseek-ai/dsh-sandbox`、`dsh-sandbox-local`、`dsh-sandbox-policy`、`dsh-fs-sandbox`、`dsh-http-proxy`、`@deepseek-ai/node-addon-system`），只读检视，未做任何修改。

**一、分类位置：④ 档的"同世界约束"（same-world confinement）。**

`@deepseek-ai/dsh-sandbox` 包 README 的 Summary 原文（作者自述，权威）：

> "Use `dsh-sandbox` to run a subprocess and everything it spawns under a per-call file-access policy. A command can run without writes (`read-only`), write only inside its workspace (`workspace-write`), or run unrestricted (`danger-full-access`). If the requested mode cannot be enforced, the call fails with `SANDBOX_UNAVAILABLE` instead of running unconfined. **This is same-world confinement: the process still shares the host kernel and filesystem; use a container, microVM, or remote executor when the whole environment must be isolated.**"

**二、三种模式 + fail-closed 语义。**

| 模式 | 含义 | 默认 |
|---|---|---|
| `read-only` | 只读（无写入） | **是（fail-safe 默认）** |
| `workspace-write` | 只允许写 workspace 内 | 需部署显式 opt-in |
| `danger-full-access` | 不限制 | 需人工审批（"After a denied call, the model can request one strictly wider mode for human approval"） |

**关键设计**：模式**无法被强制实现时，调用失败并返回 `SANDBOX_UNAVAILABLE`，而不是无约束地运行**。这是一个很好的工程取舍——**fail closed 而非 fail open**。

**三、平台 runner 链（`dsh-sandbox-local`）：**

```
Linux  : bwrap（首选，mount profile 最贴近 mode 语义） → landlock（后备）
macOS  : sandbox-exec（Seatbelt / SBPL）——唯一选择
Windows: ACL restricted-token runner
```

- **bwrap profile**：`--ro-bind / /`、`--dev /dev`、`--unshare-pid`、`--proc /proc`、`--die-with-parent`；`workspace-write` 追加 `--tmpfs /tmp` 与 `--bind <workspaceRoot> <workspaceRoot>`。
- **Landlock launcher**（`@deepseek-ai/node-addon-system` 的 `bin/landlock-run`，C 源码随包发布供审计）：
  - CLI 契约：`landlock-run [--ro <path>]... [--rw <path>]... -- <argv>...`，另有 `--probe`；
  - `--ro` 授予路径**之下**的 read+execute，`--rw` 授予完整文件系统访问（白名单模型，未列出即拒绝）；
  - profile：`--ro /` + `--rw /dev/null`（`workspace-write` 时追加 `/tmp` 与 workspaceRoot）；
  - 源码中 `MAX_ABI = 5`，并按运行时 ABI 协商裁剪访问位（ABI 1 掩码、`FS_REFER` 自 ABI 2、`TRUNCATE` 自 ABI 3、`IOCTL_DEV` 自 ABI 5）；
  - 失败协议：**exit 125 + stderr 前缀 `landlock-run: `**，使执行器能区分"启动器失败"与"命令失败"；内核不支持时**不 exec 命令**（fail closed）；
  - 老 ABI 上的**部分（best-effort）执行被接受**，并在 stderr 打印 informational 行 `landlock-run: partial enforcement (older Landlock ABI)`。**本次会话中该行实际出现**，说明当前宿主内核的 Landlock ABI 低于该构建已知的最高版本，部分文件效果承诺被降级为 best-effort。
- **Seatbelt profile**（SBPL）：`(version 1)(allow default)(deny file-write*)` + 对 `/dev/null` 与可写根逐条 `(allow file-write* (subpath ...))`。**注意官方注释：Apple 将该 CLI 标记为 deprecated，但仍随每个 macOS 发布**；其功能探针就是为了"如果某天它消失，能 fail closed"。
- **Windows ACL rung**：每 workspace 一个写 SID + 每 live session 一个**随机私有临时目录**及其派生 capability；workspace 根的 ACE 每 workspace 只物化一次并被跨会话复用（"the exact-ACE skip makes every later provision O(1)"），私有临时 ACE 在 dispose 时**撤销**。**自述为 partial enforcement**，原因有二：`WRITE_RESTRICTED` 必须在 restricting list 中保留 Everyone；**NTFS 硬链接会让同一文件对象在不同路径下别名化**。
- **`dsh-fs-sandbox`**：进程内的文件系统隔离栅栏，与 Seatbelt 通过共享的 `writableRoots` helper 取得可写根，**确保两者不会漂移**（"so the Seatbelt grant and the in-process fs fence can never drift apart"）。

**四、网络出口（`dsh-http-proxy`）：**

- 由 `dsh` launcher 在首个插件加载前，从标准代理环境变量（`HTTPS_PROXY` 等）解析并**安装进程级**的出口代理策略；`fetch()` 与任何走 `globalThis.fetch` 的 SDK 自动被代理。
- **自述的两个缺口**：**自带 transport 的 SDK 不走代理**——"two of the ones this repository ships turned out to: the OTLP exporter posts through `node:http`, and the **E2B SDK constructs its own undici dispatcher**"。OTLP 被**有意**保持直连；E2B 由调用方传入 `route.proxy`。
- 工程约束：仓库加了一条 `verify-no-bare-dispatcher` 检查，禁止在包外 `new Agent(...)` 作为 dispatcher 从而静默绕过代理；每个出口调用点必须配一个 `egress.spec.ts` 做真实路径断言。

### DSH 沙箱与上述方案的差距（结构化）

| 维度 | DSH 沙箱（④ 档） | ② 档 Agent 沙箱（E2B / AgentENV / CubeSandbox） | 差距性质 |
|---|---|---|---|
| **隔离边界** | 共享宿主内核与文件系统（作者自述） | 独立 guest 内核 + KVM + 硬件虚拟化 | **结构性**，不可通过调参弥补 |
| **能否跑异构环境** | 否 | 是 | 结构性 |
| **内核漏洞逃逸** | 受宿主内核漏洞支配；Landlock 明确不覆盖 `chmod`/`chown`/`stat`/`fcntl`/`access` 等 | 需再破 VMM + KVM + CPU | 结构性 |
| **网络出口** | 进程级代理，**自带 transport 的调用可绕过**；bwrap profile **无 `--unshare-net`** | 独立 netns / CNI 策略 / 出口网关；但 Firecracker 官方也明确不过滤流量，需宿主层配合 | **架构缺口**，DSH 侧更明显 |
| **资源限额** | 依赖宿主侧限额（本文件未在源码中核实 cgroup 配置细节 —— **未能证实**） | guest 内 cgroup + VMM 层限额 | 待核实 |
| **快照 / pause-resume** | **无**（进程对象，非可快照的 VM） | **核心能力**：CubeSandbox 百毫秒级 snapshot/clone/rollback + 跨节点 pause/resume；AgentENV 内存 CoW + fork | **能力缺失**，非性能差距 |
| **启动延迟** | **最快**（无 VM 启动；bwrap/landlock 只是进程包装） | 50~125 ms（官方数字见前） | DSH **占优** |
| **内存开销** | **最低**（无 VMM、无 guest 内核） | 单实例 < 5 MB（不含 guest 内核与 workload 内存） | DSH **占优** |
| **单机密度** | **最高** | 高（CubeSandbox 称每节点数千） | DSH **占优** |
| **启用方式** | 每个 bash/文件/终端调用**独立的模式解析与包装**，无需预置 VM 池 | 需要 warm pool / 镜像分发 / 节点调度作为前置设施 | DSH **运维负担低得多** |

**结论**：DSH 的沙箱与 E2B/AgentENV/CubeSandbox **不是竞争关系**，而是**不同层级的取舍**。DSH 选择了"零基础设施、极低延迟、fail-closed 的路径级约束"，代价是**放弃了内核级隔离、网络隔离与快照能力**。要做"多租户不可信 Agent 执行平台"，需要的是在 DSH 之下或之外补一层 ② 档设施（这也正是本仓库 `sandbox_research.md` 把 Sandbox 定位为待合作确认的核心问题的原因）。

### Agent 沙箱的性能诉求排序与各方案取舍

本仓库 `sandbox_research.md` 已把待确认的核心性能痛点列为（原文顺序）：**启动延迟 / 并发实例数 / pause-resume 时间 / 内存占用**。按这一排序看各方案：

| 优先级 | 诉求 | 谁最强 | 谁最弱 | 取舍要点 |
|---|---|---|---|---|
| 1 | **启动延迟** | ④ 容器（~10 ms 量级）／⑤ WASM（亚毫秒~数十毫秒级） | ② 通用 VM（秒级） | ② 档靠 **warm pool + 内存快照 resume**（AgentENV）或 **reflink 预热 + VMM snapshot**（CubeSandbox）把冷启动压到 50~60 ms；**③ 档拿到的是"介于两者之间但更接近 ④"的延迟** |
| 2 | **并发实例数** | ④（仅受 cgroup 限额） | ① 机密计算（受加密引擎与内存带宽约束） | CubeSandbox 单节点"数千"、Firecracker"仅受硬件资源限制"、gVisor 每实例多约 20 MiB —— **20 MiB × 1000 = 20 GB，是会算错的量级** |
| 3 | **pause-resume** | ② 档（唯一真正支持 VM 状态挂起/跨节点恢复） | ③④⑤（进程状态无法真正"暂停到存储再恢复"） | CubeSandbox 支持跨节点 pause/resume（S3 后端）；`sandbox_research.md` 已提出"暂停时内存压缩后不落盘、加快 resume"的优化方向，并指出压缩可用 SVE 加速 |
| 4 | **内存占用** | ④（近零） | ② 通用 VM | 单实例 < 5 MB 是 ② 档的及格线；**但这不含 guest 内核与 workload 内存**——计算总占用时必须把它们算进去 |

**三条容易踩的坑：**

- **把"VMM 开销"当成"实例总内存"。** Firecracker 的 `≤ 5 MiB` 是 **VMM 线程**开销，不含你配给 VM 的 128 MiB 与 guest 内核。
- **用单并发数字承诺高并发 SLA。** CubeSandbox 自己的数据就说明问题：单并发 60 ms → 50 并发 P99 137 ms。**并发一上来，长尾先崩。**
- **忘记 resume 路径的 I/O 成本。** pause-resume 的瓶颈往往不在 CPU 而在**快照读写**（内存快照落盘/传输、reflink 元数据、跨节点 S3 往返）。`sandbox_research.md` 提出的 NUMA 亲和、页大小、大块 memcpy 优化都属于这一类。

---

## 常见误区

### 误区 1：「容器就是沙箱」

**澄清**：容器是**打包与资源隔离**机制，默认**不是安全边界**。三个权威佐证：

- **NVD 对 CVE-2019-5736**（发布 2019-02-11）的描述原文：runc 至 1.0-rc6（Docker 18.09.2 之前）"allows attackers to **overwrite the host runc binary (and consequently obtain host root access)**"，手段是"在一个以 root 执行命令的容器中"利用该能力。
  来源：[NVD CVE-2019-5736](https://nvd.nist.gov/vuln/detail/CVE-2019-5736)（访问 2026-09-16）
- **NVD 对 CVE-2022-0492**（发布 2022-03-03）：内核 `cgroup_release_agent_write`（`kernel/cgroup/cgroup-v1.c`）缺陷，"allows the use of the **cgroups v1 release_agent feature to escalate privileges and bypass the namespace isolation** unexpectedly"。
  来源：[NVD CVE-2022-0492](https://nvd.nist.gov/vuln/detail/CVE-2022-0492)（访问 2026-09-16）
- **NVD 对 CVE-2024-21626**（发布 2024-01-31）：runc 1.1.11 及更早，"due to an **internal file descriptor leak**, an attacker could cause a newly-spawned container process (from `runc exec`) to have a **working directory in the host filesystem namespace**, allowing for a **container escape**"。
  来源：[NVD CVE-2024-21626](https://nvd.nist.gov/vuln/detail/CVE-2024-21626)、[GitHub Advisory GHSA-xr7r-f8xq-vfvv](https://github.com/opencontainers/runc/security/advisories/GHSA-xr7r-f8xq-vfvv)（访问 2026-09-16）

**正确表述**：容器可以作为"**纵深防御的一层**"和"**资源隔离**"，但**不能作为"防恶意多租户"的主边界**。要安全边界，请上 ② 或 ③ 档。DSH 自己的 README 也是这个立场（"use a container, microVM, or remote executor when the whole environment must be isolated"）——**注意它把 container 和 microVM 并列为"隔离整个环境"的选项，这是产品定位上的宽松表述，安全上不等价**。

### 误区 2：「WASM 天生安全」

**澄清**：WASM 规范的沙箱性质是真实的，但**不等于"跑 WASM 就安全"**：

- **规范提供的是"内存与调用栈隔离 + 能力模型"**，Wasmtime 官方列出：调用栈不可访问（返回地址与溢出寄存器不在应用可访问内存中，传统栈溢出攻击不可能）；指针编译为线性内存偏移且**所有访问检查边界**；所有控制转移都是已知且类型检查的目标；**与外界一切交互都通过 import/export，没有对系统调用或任何 I/O 的原始访问**；不存在未定义行为。
- **但这只覆盖"WASM 实例不能越界"**。剩下的全部风险在别处：
  - **宿主函数（import）**——实例能做的**恰好是宿主显式链接给它的那些**，所以宿主给的 API 有多大，攻击面就有多大；
  - **JIT / 验证器缺陷**——Wasmtime 官方专列"Defense-in-depth"章节，包括默认给线性内存前置 **2 GB guard region**、用 guard page 检测原生栈溢出、实例结束后清零线性内存/表/实例信息以防跨实例信息泄漏、选择 Rust 作为实现语言，以及正在推进的 CFI（PAuth/BTI）；
  - **侧信道**——官方单列 Spectre 章节（对 `call_indirect` 表项边界检查做缓解、`br_table` 缓解、默认依赖 page fault 检测越界因而**不插入边界检查**、配置为 dynamic memory 时才插入 Spectre 缓解；**aarch64 上 `csdb` 默认关闭因性能代价显著**）。官方原文承认："Mitigating Spectre continues to be a subject of ongoing research."
  - **终端转义序列**——官方甚至专门处理"不可信代码打印到终端"这一攻击面（把转义序列翻译成惰性替换序列）。
  来源：[Wasmtime Security](https://docs.wasmtime.dev/security.html)（访问 2026-09-16）

**正确表述**：WASM 是"**进程内能力沙箱**"，不是"**系统级安全边界**"。用它做 Agent 代码执行时，**宿主函数的设计与授能范围才是真正的安全边界**。

### 误区 3：「seccomp 能挡住内核漏洞」

**澄清**：**不能。** seccomp-bpf 只能决定"哪些系统调用被允许、以及（极其有限的）参数约束"，而内核漏洞往往**发生在被允许的系统调用内部**：

- **crosvm 官方 seccomp 文档原文**："The syntax is simple: one syscall per line, followed by a colon `:`, followed by a boolean expression used to constrain the arguments of the syscall. … **A major limitation is that checking the contents of pointers isn't possible using minijail's policy format.**"——即 **seccomp 无法检查指针指向的内容**，因此"允许 `ioctl(fd, cmd, ptr)`"就等于把 `ptr` 后面的全部语义交给内核去解析，而漏洞正好藏在解析逻辑里。
  来源：[crosvm Seccomp](https://crosvm.dev/book/appendix/seccomp.html)（访问 2026-09-16）
- **seccomp 自身还有可测量的开销**：gVisor 官方 systrap 博客在解释为何要绕开 seccomp 时引用 LWN《A seccomp overview》，指出 seccomp 过滤器开销"not insubstantial"，并说明 systrap 的优化点之一正是"避免先执行 seccomp 过滤器再生成完整信号栈"。
  来源：[Releasing Systrap](https://gvisor.dev/blog/2023/04/28/systrap-release/)、[LWN 656307](https://lwn.net/Articles/656307/)（访问 2026-09-16）
- **同一逻辑也适用于 Landlock**：Landlock 官方明确列出**当前无法限制**的操作（`chdir`、`stat`、`flock`、`chmod`、`chown`、`setxattr`、`utime`、`fcntl`、`access`）——**"用了 Landlock 就安全"同样不成立**。
  来源：[Landlock 内核文档](https://docs.kernel.org/userspace-api/landlock.html)（访问 2026-09-16）

**正确表述**：seccomp 的价值是**缩小被攻击的内核代码面**（把 300+ 系统调用砍到 40 个），从而**降低可利用漏洞的数量**；它**不能**把已允许路径上的内核漏洞变成不可利用。Firecracker 的做法（"默认过滤器只允许 Firecracker 正常运行所需的**最小**系统调用集与参数"）正是这一思路的正面示范，**但它同时也是 Firecracker 需要 KVM 边界作为第一层的原因**。

### 误区 4：「gVisor 兼容性无忧」

**澄清**：gVisor 是**独立实现的系统调用面**，官方自己把代价分成两类，并明确第二类不会自动消失：

- **结构性成本（structural costs）**：来自 Sentry 的存在——额外的内存、系统调用必须穿过更多软件层；官方直言"Sentry 的实现语言在安全上有优势，但可能尚不具备其他选择的原始性能"。**这部分是设计固有的。**
- **实现成本（implementation costs）**：某些子系统/系统调用不如成熟实现优化。官方举例**网络栈**——"continues to evolve but **does not support all the advanced recovery mechanisms** offered by other stacks and is **less CPU efficient**"。

其他必须写明的边界：

- **平台决定一切**：ptrace 平台"works everywhere … but suffers from the highest structural costs by far"；KVM 平台"runs poorly with nested virtualization"（且部分硬件如 ARM、部分云环境不支持嵌套）。
- **官方自述"并非适合所有负载"**：官方原文甚至举例"对可信数据库而言沙箱可能收益极小，因为**用户数据本来就在沙箱里**，攻击者根本不需要逃逸"。
- **官方性能页的默认口径是 ptrace**：性能指南原文明确"Except where specified, all tests below are conducted with the `ptrace` platform … **This platform is used to provide a clear understanding of the performance model, but in no way represents an ideal scenario**；users should use Systrap for best performance in most cases."——**看到"gVisor 慢 20 倍"的引用时，必须先确认它说的是哪个平台。**
  来源：[gVisor Performance Guide](https://gvisor.dev/docs/architecture_guide/performance/)（访问 2026-09-16）
- 具体"哪些系统调用/特性不支持"的清单，gVisor 有独立的 Linux/amd64 与 Linux/arm64 兼容性页面，**本轮未逐一核对 —— 具体缺口清单未能证实**，写汇总稿前应逐项核对。

### 误区 5：「microVM 就是安全的，不需要再加固」

**澄清**：microVM 提高了逃逸成本，但**不解决外联、不解决配置错误、不解决侧信道**：

- **官方自述不做流量过滤**：Firecracker 设计文档原文"Firecracker does not perform any network traffic filtering. **All egress traffic from a guest is therefore considered untrusted, and should be filtered at the host-level.**"——**把不可信 Agent 放进 microVM 但不控出口，等于给了一台能自由外联的机器。**
- **设备模型的攻击面并未消失**：Firecracker 的手段是**极小化**（文档原文把 containment 描述为"nesting several trust zones"，从 guest vCPU 线程逐级升到宿主），并叠加 seccomp + cgroups + namespaces + jailer 降权 + 只允许"特权第三方授予的资源"（如把文件拷进 chroot 或传 fd）。这是一套**纵深防御**，而不是"虚拟化本身够了"。
  来源：[Firecracker design.md](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)（访问 2026-09-16）
- **Kata 的场景也是同理**：Kata 官方把 hypervisor 进程本身也放进独立 cgroup、独立 network namespace，并按 OCI 配置给它 SELinux label——**说明"VM 边界"之外仍需进程级加固**。

### 误区 6：「有了沙箱就不需要审计与出口控制」（与本仓库相关）

**澄清**：DSH 的实现本身就是最好的反例——它同时提供 (a) 文件路径约束（bwrap/Landlock/Seatbelt/Windows ACL）、(b) 失败时 fail-closed 的 `SANDBOX_UNAVAILABLE`、(c) 一次严格更宽的模式的**人工审批**流程、(d) 出口代理策略与每个出口调用点的 `egress.spec.ts` 断言。**这四件事没有一件是"沙箱"本身能替代的**：沙箱管的是"能碰什么"，审批管的是"谁授权"，出口控制管的是"数据能不能出去"，审计管的是"事后能不能查"。

### 误区 7：「用了机密计算，数据就绝对安全了」

**澄清**：本稿 ① 档已核实的事实恰好构成这条误区的完整反驳，**而且每一条都有出处**：

- **SGX 的威胁模型从设计上就排除了侧信道。** SoK 综述（arXiv:2512.22090）指出 SGX 允许攻击者控制 OS/hypervisor/BIOS 乃至部分硬件，但**明确排除侧信道攻击**——而这正是历史上多次实际攻破的入口。**"设计上把某类攻击排除在威胁模型外"不等于"这类攻击不会发生"。**
- **证明链本身可以被攻破。** SGAxe / CacheOut（CVE-2020-0549）**从 Intel 签名的 quoting enclave 中提取了 attestation 私钥并伪造 quote**，且在侧信道缓解全部开启、系统被 Intel 视为 trusted 的状态下仍然成功。**attestation 是机密计算信任链的根，而这个根被攻破过。**
- **架构性缺陷不受"非瞬态执行"保护。** ÆPIC Leak（CVE-2022-21233）是**架构性**泄露（不是瞬态执行推测），CacheWarp（CVE-2023-20592）是**架构性软件故障注入**（利用 `INVD` 让 guest 以 stale 数据继续执行，可恢复 RSA 私钥、免认证登录 OpenSSH、经 sudo 提权 root）。**"根因在硬件"意味着缓解困难。**
- **完整性保护也可以被物理攻破。** BadRAM（IEEE S&P'25）只需**短暂物理接触内存条的 SPD 芯片**（低成本装置），即可在 DDR4/DDR5 上制造物理地址别名，**破坏 SEV-SNP 的完整性并最终攻破其 attestation**。（注：该论文报告 TDX 与 Scalable SGX 具备专用别名检测，目前不受此攻击影响。）
- **厂商自己列了"不保护什么"。** SEV 系列的官方口径明确把**可用性/资源控制**（hypervisor 可随时终止或饿死 guest）、**侧信道与时序泄露**、guest 自身软件漏洞、侵入式物理攻击、以及**信任锚（AMD 制造 / ASP 固件 / 密钥分发）被攻破**列为 out of scope。**旧版 SEV/SEV-ES 完全没有完整性保护。**
- **术语级纠错**：**CVE-2020-12966 ≠ SEVered**。SEVered（EuroSec'18）是设计/架构类攻击、**没有 CVE**；CVE-2020-12966 是 AMD SB-1013 的信息泄露问题。两者常被混为一谈，写汇总稿时务必区分。
- **能力边界也要说清**：Apple Secure Enclave **不是通用沙箱**——开发者只能通过受限接口把密钥"托付"给它（`SecKey` + Secure Enclave 保护），**不能把任意代码搬进去执行**。把它与 SGX/TDX/SEV 并列成"通用 TEE"是错的。

**正确表述**：机密计算把**信任边界**从"云厂商的运维人员"收缩到"CPU + 固件/微码 + 厂商供应链"，这是一个真实且重要的改进；但它**不提供绝对安全**，且引入了一类新的失败模式——**信任根的可用性与正确性**（attestation 被伪造、TCB 版本被回滚、微码更新需要重启等）。**如果威胁模型里包含具备物理接触能力或有能力控制微码的对手，机密计算并不能给出保证。**

**给汇总稿的表述纪律**：不要把"沙箱"当成一个单数名词来承诺能力。始终写明**是哪一档**、**是否控出口**、**是否可快照**、**失败时是 fail-open 还是 fail-closed**。


---

## 未决问题

1. **机密计算的"运行时开销百分比"仍全部缺失。** 本稿已取到 TDX 的 PAMT 内存开销（`1/256` 系统内存 / 约 `0.4%`，Dynamic PAMT 约 `0.004%`），但 **TDX、SEV-SNP 相对裸机的启动时间与吞吐损耗百分比仍无可核验数字**（SIGMETRICS'25 的实证论文 PDF 两次抓取失败）。**需要按统一测试口径重取。**
2. **① 档的攻击清单已大幅补齐，但仍有缺口。** 已核实：Plundervolt / SGAxe / CacheOut / ÆPIC Leak（SGX）、SEVered / CVE-2020-12966 / CacheWarp / BadRAM（SEV 系列）。**缺口**：CVE-2020-12966 的具体触发机制未证实（AMD 公告页抓取失败）；Arm CCA 的**实际部署成熟度**与真实攻击面未见公开材料（目前只有架构文档）。
3. **Kata Containers 的启动数字无从取得。** Kata 是框架，启动时间由 hypervisor 决定。**建议在汇总稿里明确写"Kata 无单一数字"，而不是引用二手数字。**
4. **E2B / Daytona / microsandbox 的数字虽已取得，但质量参差。** E2B 的 `~150 ms` 与 `~80 ms`、Daytona 的 `< 90 ms`、microsandbox 的 `< 100 ms`（仅注"M1 机器"）**都是无完整测试条件的厂商声明**；microsandbox 的**每实例内存开销官方完全未给**。若汇总稿要把这些数字并列成表，**必须加可信度列**，否则会误导读者以为它们与 CubeSandbox benchmark 同级。
5. **CubeSandbox 自身存在 README 与 benchmark 报告的口径冲突**（内存 `5 MB` vs `~21.5/25.7 MB`；50 并发 `67 ms` vs `276.1 ms`）。**需要向项目方或后续版本确认口径定义**，或永久保持"两个数字并列引用"的写法。
6. **AgentENV 的 fork 上限（≤16）与 `9.6×` overcommit 的测量条件未见公开说明。** 官方博客确认了"已用于 Kimi K3 的 agentic RL 训练"与成本降低区间（`88.6%–96.8%`），但后两个数字的具体口径待补。
7. **Daytona 的隔离档位存在官方表述内部张力。** 文档同时出现 "dedicated kernel"（营销）与"默认是 Linux 容器"（技术），**容器沙箱不提供独立内核**。这是产品对比中最容易被误读的一处，**汇总稿若引用 Daytona 必须明确写"默认容器、非 microVM"**。
8. **DSH 沙箱的资源限额实现细节未核实。** 本轮只在源码中确认了**文件效果策略**与**出口代理策略**，**没有**确认是否存在 cgroup / rlimit / 进程数限额的强制。**若没有，则"资源限额"这一 Agent 沙箱核心诉求在 DSH 当前实现中完全依赖宿主侧兜底。**
9. **DSH 的 Linux 路径实际选中了哪个 runner 未确认。** 源码定义了 bwrap → landlock 的优先级链与功能性探测，但**本次会话观察到的是 `landlock-run` 的 informational 行**（`partial enforcement (older Landlock ABI)`），这与"bwrap 被优先选中"的预期不完全一致。**需要确认实际选中逻辑与宿主 bwrap 可用性。**
10. **Java SecurityManager 与 .NET CAS 的确切弃用里程碑未核实**（JEP 编号、目标 JDK 版本、.NET 版本）。**⑤ 档这一行的具体表述待补。**
11. **Android / iOS 应用沙箱的一手机制原文未取到。** source.android.com 页面抓取被导航栏截断（URL 已确认，正文未取），Apple Platform Security 指南未抓取正文。
12. **⑤ 档的性能数字（WASM 实例化延迟、浏览器沙箱开销）全部缺失。**
13. **OpenAI Codex CLI 的官方安全说明原文未取到**（`developers.openai.com/codex/security` 返回 403）。本稿的 Codex 结论来自源码（`seatbelt.rs`、`linux-sandbox/landlock.rs`），**官方口径待补**。
14. **本文件与 `07` / `08` / `09` 号稿的接口约定：** 本文负责"类型学 + 沙箱/虚拟化对照 + Agent 沙箱 + 误区"；**不重复** `07` 的存量笔记映射，**不重复** `08` 的高风险项验证（鲲鹏嵌套虚拟化、软件虚拟化损耗），**不重复** `09` 的综合与趋势。若汇总稿需要某一数字，**以本文的"测试条件"列为准**，不要只搬数字。

---

## 参考资料

### 官方一手文档（本轮实际抓取成功）

| 来源 | URL | 访问日期 | 本轮取得的关键内容 |
|---|---|---|---|
| Firecracker SPECIFICATION | https://github.com/firecracker-microvm/firecracker/blob/main/SPECIFICATION.md | 2026-09-16 | `≤125 ms` boot、`≤5 MiB` overhead、`8 CPU ms` VMM 启动、测试机型 |
| Firecracker design.md | https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md | 2026-09-16 | 5 microVMs/核/秒、jailer + seccomp + cgroup 纵深防御、**不做流量过滤** |
| gVisor Performance Guide | https://gvisor.dev/docs/architecture_guide/performance/ | 2026-09-16 | structural vs implementation costs、ptrace 为默认口径的说明、CPU/内存无额外开销 |
| gVisor `syscall.csv` | https://gvisor.dev/performance/syscall.csv | 2026-09-16 | `runc 1939ns` / `runsc 38219ns` / `runsc-kvm 763ns` |
| gVisor `density.csv` | https://gvisor.dev/performance/density.csv | 2026-09-16 | 空容器 `4.09MB` vs `23.70MB` 等四组内存数据 |
| gVisor `startup.csv` | https://gvisor.dev/performance/startup.csv | 2026-09-16 | Docker 路径下 runc 1193 ms / runsc 1144 ms（empty） |
| gVisor 官方博客：Releasing Systrap | https://gvisor.dev/blog/2023/04/28/systrap-release/ | 2026-09-16 | 三平台差异、`jmp *%gs` 跳板、ARM 无此优化、2023-09 取代 ptrace |
| Kata Containers Architecture | https://github.com/kata-containers/kata-containers/blob/main/docs/design/architecture/README.md | 2026-09-16 | 每 Pod 一 VM、guest 内仍用 cgroup+namespace、shim v2、DAX |
| Cloud Hypervisor README | https://github.com/cloud-hypervisor/cloud-hypervisor | 2026-09-16 | 定位（通用 Cloud Workloads）、rust-vmm、与 Firecracker/crosvm 的差异 |
| StratoVirt 介绍（openEuler 官方） | https://docs.openeuler.openatom.cn/zh/docs/21.09/docs/StratoVirt/StratoVirt%E4%BB%8B%E7%BB%8D.html | 2026-09-16 | microvm `50 ms` 内启动、内存 `<4 MB`、运行时 syscall `<46` |
| CubeSandbox README | https://github.com/TencentCloud/CubeSandbox | 2026-09-16 | `<60 ms` 冷启动宣称、`<5 MB` 开销宣称、每节点数千、百毫秒级快照 |
| **CubeSandbox 官方 Benchmark 报告** | https://github.com/TencentCloud/CubeSandbox/blob/master/docs/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md | 2026-09-16 | **条件完整的一组实测数据**：串行 47.8 ms、50 并发 276.1 ms、摊销内存 21.5~25.7 MB、快照 ~50 ms / 从快照 60–85 ms / rollback ~82 ms |
| Chromium sandbox README | https://chromium.googlesource.com/chromium/src/+/main/sandbox/README.md | 2026-09-16 | 三平台机制（Seatbelt / namespaces+Seccomp-BPF / restricted token+job object+desktop+integrity） |
| Wasmtime Security | https://docs.wasmtime.dev/security.html | 2026-09-16 | 线性内存/能力模型、防御纵深（2GB guard、清零、Rust、CFI）、Spectre 缓解与未解决项、终端转义 |
| crosvm Seccomp | https://crosvm.dev/book/appendix/seccomp.html | 2026-09-16 | **无法检查指针内容**、每设备每架构独立策略、编译期内嵌 BPF |
| crosvm Sandboxing | https://crosvm.dev/book/appendix/sandboxing.html | 2026-09-16 | 每个虚拟设备独立进程 + 按需 FD |
| Landlock 内核文档 | https://docs.kernel.org/userspace-api/landlock.html | 2026-09-16 | ABI 1~11 能力表、**无法限制的 syscall 清单**、OverlayFS 语义不合、可叠加不可移除 |
| bubblewrap README | https://github.com/containers/bubblewrap | 2026-09-16 | **"不是完整沙箱"**、`--new-session`/TIOCSTI、挂载即提权面、策略反噬应用沙箱 |
| systemd.exec(5) | https://man7.org/linux/man-pages/man5/systemd.exec.5.html | 2026-09-16 | 沙箱化选项全表、`RootEphemeral`/`RootImage`/dm-verity、**AF_UNIX 不受只读影响**、用户服务中不可用 |
| MSIX containerization overview | https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview | 2026-09-16 | **非 VM、无需独立镜像**、Full trust vs AppContainer、虚拟化范围 |
| Intel SGX 官方页 | https://www.intel.com/content/www/us/en/architecture-and-technology/software-guard-extensions.html | 2026-09-16 | enclave 内隔离、"数据中心最小信任边界"、远程证明 |
| Intel TDX 官方页 | https://www.intel.com/content/www/us/en/developer/tools/trust-domain-extensions/overview.html | 2026-09-16 | 仅能力总览（细节未取得） |
| Arm CCA 官方页 | https://www.arm.com/architecture/security-features/arm-confidential-compute-architecture | 2026-09-16 | Armv9-A、Realms、保护敏感数据与代码 |
| NVD CVE-2019-5736 | https://nvd.nist.gov/vuln/detail/CVE-2019-5736 | 2026-09-16 | runc ≤1.0-rc6 覆盖宿主 runc 二进制 → 宿主 root |
| NVD CVE-2022-0492 | https://nvd.nist.gov/vuln/detail/CVE-2022-0492 | 2026-09-16 | cgroup v1 `release_agent` 提权 + 绕过 namespace 隔离 |
| NVD CVE-2024-21626 | https://nvd.nist.gov/vuln/detail/CVE-2024-21626 | 2026-09-16 | runc ≤1.1.11 fd 泄漏 → 工作目录落在宿主 namespace → 逃逸 |
| runc GHSA-xr7r-f8xq-vfvv | https://github.com/opencontainers/runc/security/advisories/GHSA-xr7r-f8xq-vfvv | 2026-09-16 | "several container breakouts due to internally leaked fds" |
| Manco et al., SOSP'17（博客转述） | https://blog.acolyer.org/2017/11/02/my-vm-is-lighter-and-safer-than-your-container/ | 2026-09-16 | unikernel VM `4 ms`、fork/exec `~1 ms`、Docker `~150 ms`、LightVM save `~30 ms`/restore `~20 ms`、Xen `128/550 ms` |
| LWN 656307（seccomp 开销，经 gVisor 引用） | https://lwn.net/Articles/656307/ | 2026-09-16 | seccomp 过滤器开销"not insubstantial"（未读原文，经 gVisor 官方引用转述） |
| opencontainers/runc issue #142（bubblewrap CVE-2017-5226） | https://github.com/containers/bubblewrap/issues/142 | 2026-09-16 | TIOCSTI / `--new-session`（经 bubblewrap README 引用） |

### ① 档（机密计算）参考资料

| 来源 | URL | 访问日期 | 本轮取得的关键内容 |
|---|---|---|---|
| Intel SGX EPC / 世代容量 | https://www.intel.com/content/www/us/en/support/articles/000059614/processors/intel-xeon-processors.html | 2026-09-16 | EPC 容量随世代/SKU 剧变；MEE → AES-XTS；Xeon 6 双路 1 TB |
| Intel SGX 消费级弃用说明 | https://www.intel.cn/content/www/cn/zh/support/articles/000089271/intel-nuc.html | 2026-09-16 | SGX 自第 11 代 Core 起不再支持 |
| Plundervolt 官网 | https://plundervolt.com/ | 2026-09-16 | CVE-2019-11157，故障注入破坏 enclave 完整性 |
| SGAxe / CacheOut | https://sgaxe.com/ | 2026-09-16 | CVE-2020-0549，提取 quoting enclave 的 attestation 私钥；演示条件与 40 s/10 s 数字 |
| ÆPIC Leak | https://aepicleak.com/ | 2026-09-16 | CVE-2022-21233，APIC MMIO 架构性 stale 数据泄露 |
| Linux 内核 TDX 文档 | https://docs.kernel.org/arch/x86/tdx.html | 2026-09-16 | SEAM/TME-MK/`#VE`、PAMT `1/256` 系统内存（dmesg 示例）、证明依赖 SGX Quoting Enclave、无法在 S3 存活 |
| TDX Dynamic PAMT 补丁 | https://lkml.iu.edu/hypermail/linux/kernel/2506.1/01490.html | 2026-09-16 | 静态 PAMT 约 `0.4%` 系统内存 → Dynamic PAMT 约 `0.004%` |
| **AMD SEV-SNP Primer**（arXiv:2608.04039） | https://arxiv.org/abs/2608.04039 ｜ [HTML](https://arxiv.org/html/2608.04039v1) | 2026-09-16 | SME/SEV/SEV-ES/SNP 覆盖对照表、RMP 16 B/4 KB 表项、VMPL、AES-128-XEX vs AES-256-XTS、out-of-scope 威胁清单 |
| SEVered（EuroSec'18） | https://arxiv.org/abs/1805.09604 | 2026-09-16 | 恶意 hypervisor 通过 guest 内远程服务提取全部内存；**无 CVE** |
| AMD SB-1013（CVE-2020-12966） | https://www.amd.com/en/resources/product-security/bulletin/amd-sb-1013.html | 2026-09-16 | 公告标题确认是 SEV 信息泄露；**具体机制未证实**（页面内容未取到） |
| CacheWarp | https://cachewarpattack.com/ ｜ [USENIX Sec'24](https://www.usenix.org/conference/usenixsecurity24/presentation/zhang-ruiyi) | 2026-09-16 | CVE-2023-20592，`INVD` 架构性故障注入，影响 SEV-ES/SNP |
| BadRAM | https://badram.eu ｜ [作者页](https://luca-wilke.com/publication/ieeesp25-badram/) | 2026-09-16 | IEEE S&P'25，SPD 物理篡改→地址别名→攻破 SNP attestation；TDX/Scalable SGX 有检测 |
| Arm Learning Paths: CCA | https://learn.arm.com/learning-paths/cross-platform/cca_rme/cca/ | 2026-09-16 | 四态（Normal/Secure/Realm/Root）、Realm 可证明、RME + GPT |
| Arm Learning Paths: CCA 裸机示例 | https://learn.arm.com/learning-paths/cross-platform/cca_rme/bare-metal/ | 2026-09-16 | GPT 16 KB granule、`GPTBR_EL3`/`GPCCR_EL3`、granule 权限变更触发内存异常 |
| Arm Neoverse RD: RME | https://neoverse-reference-design.docs.arm.com/en/rd-infra-2024.09.30/_sources/shared/rme.rst.txt | 2026-09-16 | 四态编码 `SCR_EL3.{NSE,NS}`、隔离保证、TF-A/TF-RMM/KVM 软件栈、CCA HES |
| OP-TEE 官方文档 | https://optee.readthedocs.io/en/3.8.0/general/about.html | 2026-09-16 | TrustZone 之上的开源 TEE runtime；GP TEE API v1.1.x / Client API v1.0 |
| Apple Platform Security: Secure Enclave | https://support.apple.com/guide/security/secure-enclave-sec59b0b31ff/web | 2026-09-16 | 协处理器、独立 secure boot、UID 绑定密钥 |
| Apple Developer: Protecting keys with SE | https://developer.apple.com/documentation/Security/protecting-keys-with-the-secure-enclave | 2026-09-16 | 只能用受限接口托付密钥，**不是运行任意代码** |
| SoK: Abstraction of TEEs（arXiv:2512.22090） | https://ar5iv.labs.arxiv.org/html/2512.22090 | 2026-09-16 | TEE 生态年表（TrustZone 2004 / SGX 2015 / SEV 2016 / … / TDX 2021 / CoVE 2022）、app-based vs VM-based、SGXv1 EPC 128 MB、**SGX 威胁模型明确排除侧信道** |
| CCC《A Technical Analysis of Confidential Computing》v1.3 | https://confidentialcomputing.io/wp-content/uploads/sites/10/2023/03/CCC-A-Technical-Analysis-of-Confidential-Computing-v1.3_Updated_November_2022.pdf | 2026-09-16 | 业界对 confidential computing / TEE 的定义与威胁模型（未逐页核对） |
| Intel TDX Demystified（arXiv:2303.15540） | https://arxiv.org/abs/2303.15540 | 2026-09-16 | 基于公开文档与源码的 TDX 系统分析（未逐页核对） |
| CVM Explained（SIGMETRICS'25） | https://dse.in.tum.de/wp-content/uploads/2024/11/sigmetrics25summer-CVM-Explained.pdf | 2026-09-16 | **抓取失败，数值未能核验** —— 记录以免重复劳动 |
| TEE 横向基准（Computers & Security 154, 2025） | https://www.sciencedirect.com/science/article/pii/S0167404825001464 | 2026-09-16 | SGX/SEV/TDX 对比（仅确认存在性，数值未取） |

### AI Agent 沙箱参考资料

| 来源 | URL | 访问日期 | 本轮取得的关键内容 |
|---|---|---|---|
| E2B 官方文档 | https://docs.e2b.dev/ ｜ [persistence](https://docs.e2b.dev/sandbox/persistence) ｜ [snapshots](https://docs.e2b.dev/sandbox/snapshots) ｜ [fork](https://docs.e2b.dev/sandbox/fork) ｜ [FAQ](https://docs.e2b.dev/faq/sandbox-lifetime) | 2026-09-16 | Firecracker microVM、`~150 ms` / `~80 ms`（无条件）、pause 保存内存、paused 不计费不占并发、fork ≤100 |
| E2B infra 仓库 | https://github.com/e2b-dev/infra | 2026-09-16 | 运行时 Apache-2.0 开源、可自托管 |
| Daytona 沙箱文档 | https://www.daytona.io/docs/en/sandboxes.md ｜ [docs/en.md](https://www.daytona.io/docs/en.md) | 2026-09-16 | `< 90ms`（无条件）、**默认容器非 microVM**、容器沙箱不支持 pause/fork |
| microsandbox isolation | https://docs.microsandbox.dev/security/isolation ｜ [GitHub](https://github.com/superradcompany/microsandbox) | 2026-09-16 | libkrun + libkrunfw、KVM / Hypervisor.framework / WHP、无需 root、`< 100 ms`（条件：M1） |
| AgentENV 官方博客 | https://kvcache.ai/blog/agentenv-open-sourced/ ｜ [GitHub](https://github.com/kvcache-ai/AgentENV) ｜ [文档](https://kvcache-ai.github.io/AgentENV/latest/getting-started/overview.html) | 2026-09-16 | 清华 MADSys + Moonshot 联合开源、Kimi K3 agentic RL、成本降 88.6%–96.8%、overlaybd+ublk、kernel 6.8+ |
| Claude Code sandboxing | https://code.claude.com/docs/en/sandboxing | 2026-09-16 | macOS Seatbelt / Linux bubblewrap + socat 域名级网络隔离 + 可选 seccomp；Ubuntu 24.04+ 需 AppArmor profile |
| Anthropic sandbox-runtime | https://github.com/anthropic-experimental/sandbox-runtime | 2026-09-16 | 独立可复用沙箱组件，可沙箱化任意进程与 MCP server |
| OpenAI Codex 沙箱源码 | [seatbelt.rs](https://github.com/openai/codex/blob/main/codex-rs/sandboxing/src/seatbelt.rs) ｜ [landlock.rs](https://github.com/openai/codex/blob/main/codex-rs/linux-sandbox/src/landlock.rs) | 2026-09-16 | macOS Seatbelt；Linux bubblewrap + no_new_privs + seccomp，Landlock 为 legacy |
| Modal | https://modal.com/docs/guide/security ｜ [cold-start](https://modal.com/docs/guide/cold-start) | 2026-09-16 | 容器 + gVisor；boot ~1 s（无条件） |
| Fly.io Machines | https://fly.io/blog/fly-machines/ | 2026-09-16 | Firecracker；已有 Machine 远低于 1 s，首次创建低双位数秒 |
| Cloudflare Sandbox | https://developers.cloudflare.com/sandbox/concepts/architecture/ | 2026-09-16 | 每实例独立 VM；冷启动 1–3 s 区间 |
| Northflank Sandboxes | https://northflank.com/docs/v1/application/sandboxes/sandboxes-on-northflank.md | 2026-09-16 | microVM-backed container + 用户态内核；boot < 1 s |
| Vercel Sandbox | https://vercel.com/blog/vercel-sandbox-is-now-generally-available | 2026-09-16 | Linux microVM（Hive + Firecracker）；**未发布自测冷启动数字** |
| Morph | https://cloud.morph.so/docs/developers | 2026-09-16 | Infinibranch 快照/分支/恢复 < 250 ms（无条件） |
| Runloop Devbox | https://docs.runloop.ai/docs/devboxes/overview | 2026-09-16 | 隔离 VM；首次命令 "a few seconds" |
| Blaxel Sandboxes | https://github.com/blaxel-ai/docs/blob/main/Sandboxes/Overview.mdx | 2026-09-16 | standby 恢复 < 25 ms、scale-to-zero 保留内存态 |
| CodeSandbox / Together | https://docs.together.ai/docs/together-code-sandbox | 2026-09-16 | microVM；克隆/恢复 "under three seconds" |

### DSH 发布包源码（只读检视，未修改）

| 组件 | 本轮取得的证据 |
|---|---|
| `@deepseek-ai/dsh-sandbox` | 三模式词汇（read-only / workspace-write / danger-full-access）、`SANDBOX_UNAVAILABLE` fail-closed、**"same-world confinement，进程仍共享宿主内核与文件系统"** |
| `@deepseek-ai/dsh-sandbox-local` | 平台 runner 链（bwrap→landlock / sandbox-exec / windows-acl）、bwrap 与 Landlock 与 SBPL 与 Windows ACL 的具体 profile 构造、fail-closed 探测 |
| `@deepseek-ai/dsh-sandbox-policy` | 部署默认模式（fail-safe 默认 `read-only`）、会话级模式切换并写入会话日志、restart 后经 replay 保持 |
| `@deepseek-ai/dsh-fs-sandbox` | 进程内 fs 栅栏与 Seatbelt 共享 `writableRoots`，避免策略漂移 |
| `@deepseek-ai/dsh-http-proxy` | 进程级出口代理策略、仅覆盖 `fetch`/dispatcher、**E2B SDK 与 OTLP 自带 transport 绕过**、`verify-no-bare-dispatcher` 门禁、逐调用点 `egress.spec.ts` |
| `@deepseek-ai/node-addon-system`（`landlock-run`） | `--ro`/`--rw`/`--probe` CLI 契约、`MAX_ABI = 5`、按 ABI 裁剪访问位、exit 125 + `landlock-run: ` stderr 协议、老 ABI best-effort 且打印 informational 行 |

### 本仓库存量笔记（引用，未修改）

- `docs/rust-kunpeng/sandbox_research.md` —— 沙箱性能诉求优先级（启动延迟 / 并发实例数 / pause-resume / 内存占用）与优化方向清单（Euler LLVM 编译、内存压缩不落盘、大块 memcpy、NUMA 亲和、页大小、Kunpeng CPU 性能分析工具）。
- `docs/rust-kunpeng/cubesandbox-rust-analysis.md` —— CubeSandbox 五大 Rust workspace、CubeCoW 的 Flat Snapshot Model 与增量脏页跟踪、Kata agent fork、ARM64 全栈支持（v0.5）、MSRV 锁定 1.77。
- `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` —— AgentENV vs CubeSandbox 五维对比（VMM 选型 / CoW 路径 / 存储 / 集群生态 / 预热手段）与两句话总结。
- `docs/rust-kunpeng/rust-application-patterns.md` —— 结论 2（基础设施 C/C++ → Rust 迁移不可逆，含 CubeSandbox 一行）与结论 4（Rust 作为"安全边界封装层"，含 seccompiler 一行）。
- `research/virtualization/07-existing-notes-map.md` —— microVM 快照/CoW 是 KVM 架构红利；AgentENV 的 CoW 在虚拟化层、CubeSandbox 的 CoW 在文件系统层。
- `research/virtualization/08-verification-notes.md` —— 鲲鹏 920 嵌套虚拟化（FEAT_NV2 缺失）与软件虚拟化损耗分档；**本文不重复**。

---

## 补充（第三批证据已并入正式笔记，本文相应结论以正式笔记为准）

第三批"逃逸/误区"专题证据返回时本会话已合并回 `master`、写入门禁生效，故**未回写本文件**，
而是直接并入了 `docs/virtualization/vendors-and-sandboxes.md`。以下三处**本文的旧表述已被正式笔记取代**：

1. **"容器 ≠ 沙箱"**：本文原以 CVE 举证；正式笔记升级为**标准机构定性**——NIST SP 800-190 §3.5.2
   （共享内核 "larger inter-object attack surface than seen with hypervisors"；容器隔离 "not as high as that
   provided by hypervisors"）与 Kubernetes 官方多租户文档（容器是 "weaker isolation boundary"），
   并把逃逸 CVE 扩到 8 项（含 BuildKit **CVE-2024-23652，CVSS 10.0**）。
2. **"seccomp 挡不住内核漏洞"**：本文原只有 crosvm 一条旁证；正式笔记补上**内核官方文档的直接否定**
   （"System call filtering isn't a sandbox."，本次已逐字抓取核实）、**io_uring 使 seccomp 失去可见性**、
   TSYNC 竞态绕过（CVE-2026-89603）与 25%/15% 的开销实测。
3. **两处"未能证实"已可回填**：gVisor 兼容性有官方具体清单（沙箱内 cgroup 只记账不限额、不支持
   fat32/ext3/ext4 挂载、**io_uring 默认禁用**、**沙箱内跑 KVM 不受支持**、GPU 需 `--nvproxy` 且严格匹配驱动）；
   **ptrace 平台已被 systrap 取代、现不再支持并将移除**（本文原写法偏保守）。另新增 WASM 真实逃逸 CVE 与
   microVM 逃逸的表述纪律（尚无公开 guest→host 逃逸 CVE，但硬件侧信道不受 microVM 边界保护）。
