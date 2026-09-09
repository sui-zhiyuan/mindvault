# Kerberos / GSSAPI 认证机制

> 更新：2026-09-09

## 简介

GSSAPI（Generic Security Services API）是一套**与具体机制无关**的安全接口（RFC 2743/2744），让应用程序不必硬编码某种认证协议，即可完成"认证、完整性、加密"三件事。Kerberos 是 GSSAPI 下最常用的机制之一，基于"可信第三方 KDC"实现单点登录（SSO）：用户一次认证，之后登录多台服务器无需再输密码。

本文回答的核心问题：SSH 登录为何会因 GSSAPI/Kerberos 超时而缓慢，以及 KDC 的完整工作原理。

前置知识：对称加密 / 非对称加密的基本概念。

## 术语列表

| 术语 | 全称 | 含义 |
|---|---|---|
| GSSAPI | Generic Security Services API | 与机制无关的安全接口层 |
| KDC | Key Distribution Center | 认证中心，Kerberos 的"发证机关" |
| AS | Authentication Service | KDC 内验证身份、发 TGT 的组件 |
| TGS | Ticket Granting Service | KDC 内发服务票据的组件 |
| TGT | Ticket Granting Ticket | "门票"，用于向 TGS 换服务票据 |
| principal | — | 主体身份，如 `user@REALM`、`host/server@REALM` |
| realm | — | Kerberos 管理域 / 信任边界 |
| keytab | key table | 存储服务端密钥的文件 |
| SPN | Service Principal Name | 服务身份名，`host/FQDN@REALM` |
| MIC | Message Integrity Code | 消息完整性校验码 |
| PKINIT | Public Key Crypto for Initial Auth | 用证书替代密码拿 TGT 的可选扩展 |
| OID | Object Identifier | 标识 GSSAPI 后端机制的对象标识符 |
| SPNEGO | Simple and Protected GSS-API Negotiation | 协商机制，在 Kerberos/NTLM 间谈判 |
| NTLM | NT LAN Manager | 微软旧认证机制，通常经 SPNEGO 使用 |

## 核心内容

### GSSAPI 的定位：机制无关的 API 层

GSSAPI 不是网络协议，而是一层**本地库的编程接口**（RFC 2743/2744），从设计上就是"机制无关"的：

- 它只规定编程接口（`gss_init_sec_context`、`gss_accept_sec_context`、`gss_wrap` 等）和"不透明 token"概念；
- **不规定任何线上协议**——包括"如何获取 TGT"：那由具体机制定义（Kerberos 用 AS-REQ/AS-REP，见 RFC 4120）；
- token 是"搭便车"传输的：SSH 装在 `SSH_MSG_USERAUTH_GSSAPI` 消息里，HTTP 装在 `Authorization: Negotiate` 头里，GSSAPI 自己从不直接连网。

**Kerberos 不是唯一机制**，GSSAPI 通过 OID 标识后端机制：

| 机制 | 说明 |
|---|---|
| Kerberos 5 | 最常用，OID `1.2.840.113554.1.2.2` |
| SPNEGO | 协商机制，在 Kerberos 与 NTLM 间谈判（HTTP Negotiate） |
| NTLM | 微软旧机制，通常经 SPNEGO 使用 |
| IAKERB | 跨域初始认证（RFC 8062） |
| SCRAM / GS2 | 与 SASL 桥接的机制（RFC 5801/5802） |

类比：GSSAPI 之于安全机制，如同 ODBC/JDBC 之于数据库、OpenGL 之于图形驱动——统一 API 抽象不同后端，本身不占网络层，也不是 TCP 之上的应用协议（区别于 SMTP）。

### 架构：三个角色与 realm

Kerberos 引入一个**可信第三方 KDC** 做发证机关，客户端和服务端不必直接互传密码：

| 角色 | 说明 | 持有密钥 |
|---|---|---|
| 客户端 | 想登录的用户 | 只持有自己的密码（派生出的密钥） |
| KDC | 认证中心，分 AS + TGS 两部分 | 持有所有用户、所有服务的密钥 |
| 服务端 | 想访问的资源（如 SSH） | 只持有自己的服务密钥 |

**realm** 是 Kerberos 的管理域 / 信任边界——一个命名空间 + 一个 KDC（集群）。大小写敏感，惯例用大写 DNS 域名（如 `EXAMPLE.COM`），但本质只是个名字，与 DNS 无关。同 realm 内自动互信，跨 realm 需额外配置信任。

### 认证流程：三种票据交换

认证分三个阶段，对应三次"请求-响应"（REQ/REP），全程**密钥不出网**：

**① 登录拿 TGT（AS-REQ / AS-REP）** — 用户向 AS 申请 TGT：

- 客户端发 AS-REQ（声明身份 `user@REALM`）；
- AS 返回 TGT（用 TGS 密钥加密）+ 会话密钥 K1（用用户密码派生密钥加密）；
- 客户端用密码本地解密出 K1，存入凭据缓存。**密码只在本地用，不出网**。

**② 申请服务票据（TGS-REQ / TGS-REP）** — 向 TGS 换服务票据：

- 客户端发 TGT + Authenticator（用 K1 加密的时间戳）；
- TGS 解开 TGT 验证身份，返回服务票据（用**服务端密钥**加密）+ 会话密钥 K2（用 K1 加密）。

**③ 出示票据（AP-REQ / AP-REP）** — 连接真正的服务端：

- 客户端发服务票据 + Authenticator（用 K2 加密）；
- 服务端用自己的密钥解开票据得到 K2，再用 K2 验证时间戳，完成双向认证。

### 核心是对称加密，不是 RSA

**Kerberos 全程用对称加密，KDC 没有"公钥"，也不存在 RSA 验签。** 服务端判断"票据是否 KDC 签发"，靠的是"能不能用自己的密钥解开"：

- KDC 用服务端密钥 Ks 加密服务票据；
- 服务端用 keytab 里的同一把 Ks 解密；
- **能解开 = 加密者知道 Ks = 只有 KDC 和自己知道 Ks = 票据必来自 KDC**。

对称密码的精妙之处：因为密钥是双方独享的秘密，"能正确解密"同时证明了保密性和来源真实性。

> 唯一的非对称例外是 **PKINIT**（RFC 4556），只在"客户端 → AS 拿 TGT"这一步用证书替代密码，核心票据机制仍是对称的。

### 每服务独立密钥，防止伪造票据

每个服务 principal 有一把**独立密钥**，只在 KDC 和该服务自己的 keytab 里：

```
KDC 数据库:  host/serverA → Ks_A     host/serverB → Ks_B   (互不相关)
serverA keytab: 只有 Ks_A            serverB keytab: 只有 Ks_B
```

因此 serverA **无法伪造 serverB 的票据**（不知道 Ks_B，用 Ks_A 加密 B 解不开）。票据结构还明文含 `sname`（目标服务名）做双重绑定。密钥泄露被隔离在单个服务内，不会横向扩散。

> 唯一的"总开关"是 KDC 自己——它持有所有密钥，攻破 KDC 即可伪造任意票据。故 KDC 必须专用、锁定、隔离部署。

### SSH 如何使用 GSSAPI 登录（gssapi-with-mic）

SSH 的 GSSAPI 认证方法叫 `gssapi-with-mic`，把三阶段浓缩进 SSH 认证消息：

1. 客户端通过 GSSAPI 内部去 KDC 拿服务票据（TGS-REQ）；
2. 把 AP-REQ token 装进 `SSH_MSG_USERAUTH_GSSAPI` 发给服务端；
3. 服务端用 GSSAPI accept 解开票据、验证身份；
4. 对 SSH 会话做 MIC 校验，认证成功。

## 延伸

### KDC 发现方式

KDC 地址**不是服务端告诉客户端的**（SSH 协议无此字段）。客户端分两步定位，每步都有静态配置 / DNS 动态两条路：

| 步骤 | 静态（krb5.conf） | DNS 动态 |
|---|---|---|
| 定 realm | `[domain_realm]` 域名→realm | TXT 记录 `_kerberos.<域名>`，或默认域名大写 |
| 定 KDC 地址 | `[realms]` `kdc = ...` | SRV 记录 `_kerberos._udp.<realm>` |

是否用 DNS 由 `dns_lookup_realm` / `dns_lookup_kdc` 开关控制。静态配置更常见、更可靠。

### keytab 密钥安全与分发

服务密钥**不在网络上同步**，而是管理员**离线分发**：在 KDC 上 `addprinc -randkey` 生成随机密钥 → `ktadd -k` 导出成 keytab 文件 → 用安全通道（scp/U盘）搬运到服务端。

- **keytab 文件 = 密钥本身**，搬运那一下是真正的安全敏感点，须用加密通道；文件权限设 `root:root 600`。
- 认证时网络上只传"用密钥加密过的票据"，密钥本身从不出现。
- 例外：远程 `kadmin` 的 `ktadd` 会经网络传 keytab，但该管理通道本身用 Kerberos 加密保护，且与登录认证链路无关。

### SSH 登录超时排障（本次实例）

**症状**：`ssh 192.168.x.x` 登录耗时 ~12.5s，网络 RTT 仅 38ms。

**定位**：`ssh -vvv` 带时间戳逐阶段计时，发现卡在 `Next authentication method: gssapi-with-mic` 后静默 ~10.5s + ~2s（两次）。

**原因**：客户端默认开启 GSSAPI（Ubuntu 的 `/etc/ssh/ssh_config` 里 `GSSAPIAuthentication yes`），且服务端通告支持 gssapi；客户端没有 Kerberos 票据，于是试图联系一个**不存在的 KDC**（本机无 `krb5.conf`，走 DNS SRV 自动发现，查不到 → 等超时）。超时后回落 password/publickey。

**解法**：客户端 `~/.ssh/config` 加：

```
Host 192.168.x.x
  GSSAPIAuthentication no
  GSSAPIKeyExchange no
```

（或服务端 `sshd_config` 同样关闭，对所有客户端生效。）修复后连接→认证耗时 ~0.3s。

### KDC 部署安全考量

- **KDC 绝不能与业务服务器同机**：KDC 持有所有密钥，业务服务器（SSH）是攻击面最大的入口，两者同机 = 攻破一个点全盘沦陷。
- 单机、单人场景**不要**为"修 12 秒超时"而部署 Kerberos——SSH 本身全程加密，Kerberos 的真正价值只在多机 SSO / 集中管理，为单机部署是"用更大的复杂度解决已解决的小问题"。
- 直接连 IP 默认**无法**用 KDC：SPN 按主机名构造（`host/FQDN@REALM`），连 IP 会去要 `host/<IP>@REALM` 而 KDC 无此记录。可用 `-o GSSAPIServerIdentity=FQDN` 绕过。

## Q&A

- **realm 是什么？** Kerberos 的管理域/信任边界，一个命名空间 + 一个 KDC，惯例大写 DNS 域名，本质只是个名字。
- **krb5.conf 能理解成 /etc/hosts 吗？** 只对一半：`[realms]` 段确实像 hosts 一样"静态覆盖 DNS 指定地址"，但整体是多段配置文件，且定位是两级映射（域名→realm→地址），hosts 只有一级。
- **KDC 和 SSH server 间密钥会走网络吗？** 不会。密钥离线生成、导出成 keytab、再安全搬运，认证时网络只传加密票据，密钥不出网。
- **服务端持有 KDC 的公钥吗？** 否。KDC 无公钥，服务端持有自己的对称服务密钥（keytab），靠解密票据验证来源。
- **为什么单次超时约 12 秒？** GSSAPI 内部两次尝试（gssapi-with-mic 与 gssapi-keyex）各等一次 KDC 超时，合计 ~10.5s + ~2s。

## 参考资料

- RFC 4120 — The Kerberos Network Authentication Service (V5)：<https://www.rfc-editor.org/rfc/rfc4120>（2026-09-09）
- RFC 2743 — Generic Security Service Application Program Interface：<https://www.rfc-editor.org/rfc/rfc2743>（2026-09-09）
- RFC 2744 — Generic Security Service API: C-bindings：<https://www.rfc-editor.org/rfc/rfc2744>（2026-09-09）
- RFC 4556 — PKINIT：<https://www.rfc-editor.org/rfc/rfc4556>（2026-09-09）
- RFC 4178 — SPNEGO：<https://www.rfc-editor.org/rfc/rfc4178>（2026-09-09）
- RFC 8062 — IAKERB：<https://www.rfc-editor.org/rfc/rfc8062>（2026-09-09）
- RFC 5801 / 5802 — GS2 / SCRAM：<https://www.rfc-editor.org/rfc/rfc5801>（2026-09-09）
- OpenSSH man pages：`ssh_config` <https://man.openbsd.org/ssh_config>、`sshd_config` <https://man.openbsd.org/sshd_config>（2026-09-09）
- MIT Kerberos docs：<https://web.mit.edu/kerberos/>（2026-09-09）
