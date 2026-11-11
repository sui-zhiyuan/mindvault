# dsh 在 pnpm 全局安装下启动失败（No usable native binding found）

> 更新：2026-11-11

## 简介

`pnpm add -g` 在 pnpm 12.16 起默认把全局包实体放进共享的 **global virtual store**（`~/.local/share/pnpm/store/v11/links/`），安装目录里只剩符号链接。Node 跟随 symlink 解析模块时物理起点变成 store 路径，于是 pnpm 的 hoisting 目录 `node_modules/.pnpm/node_modules` 不再是任何模块的祖先目录；而 pnpm 的 `NODE_PATH` 兜底只注入**它自己 spawn 的进程**。

dsh 的原生加载器 `node-addon-native-custom-loader` 恰恰用**裸 `require()`** 解析一个它自己没声明、只挂在入口包 `optionalDependencies` 里的平台包，靠的就是 hoisting。两者相遇的结果是 `dsh web` 在 host preparation 阶段直接抛 `No usable native binding found`，而预编译回退路径也不存在，无法自愈。

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| CAS | Content-Addressable Store | pnpm 按内容哈希存文件、全机共享的仓库，默认 `~/.local/share/pnpm/store/v11` |
| virtual store | — | 把 CAS 文件硬链接成"带依赖拓扑的目录树"的那一层，默认 `<项目>/node_modules/.pnpm` |
| global virtual store | — | 虚拟存储的另一种落点：`<store>/links`，整机一份，`pnpm add -g` 的默认 |
| hoisting | — | 把未声明的依赖提升到上层 `node_modules` 供回溯解析的机制 |
| shim | — | pnpm 生成的启动壳脚本，内容形如 `exec node <pkg>/lib/bin.js` |
| prebuilt | Prebuilt Binary | 随包发布的预编译 `.node` 文件，无需现场编译 |
| ABI / napi | Application Binary Interface | 原生模块与 Node 的二进制兼容标识，此处 prebuilt 为 `napi-v9` |
| host preparation | — | dsh 启动时装配 profile、加载插件与原生模块的阶段 |

## 核心内容

### 1. 症状

升级 pnpm 后，从 shell 直接执行 `dsh web`：

```
dsh: fatal uncaught exception: Error: dsh: host preparation failed:
No usable native binding found for node-addon-require-builtin-linux-x64-gnu (auto)
  [cause]: No usable native binding found for node-addon-require-builtin-linux-x64-gnu (auto)
   at noUsableBindingError (.../node-addon-native-custom-loader/lib/index.js)
```

三个必须先钉死的前提，否则会一直在错的方向上排查：

1. `pnpm install` **本身是成功的**——平台包与 prebuilt 都在，`.modules.yaml` 的 `skipped` 列表里也没有它。
2. 失败发生在**模块解析**，不是安装、不是构建、不是 ABI 选择。
3. 报错里的 `node-v147` 只是当次进程的 ABI 标签（与 `process.versions.modules` 无关的展示串），失败发生在"选 ABI"之前。

### 2. 原因

#### 2.1 根因链

以故障当时那份安装（`dsh 0.2.1-rc.5` + `pnpm 12.16.1`）为例：

1. 全局安装目录 `~/.local/share/pnpm/global/v11/<hash>/node_modules/` 里只有符号链接，实体在 `~/.local/share/pnpm/store/v11/links/`；`.modules.yaml` 里写着 `virtualStoreDir: ../../../../store/v11/links`、`packageManager: pnpm@12.16.1`。
2. Node 跟随 symlink 到真实路径，`require()` 的解析起点变成 store 路径，因此安装目录内的 hoisted 树 `node_modules/.pnpm/node_modules` **不再位于任何模块的祖先链上**。
3. `node-addon-native-custom-loader` 用裸模块名 `require('node-addon-require-builtin-linux-x64-gnu')`（`lib/index.js:454`）从自身位置解析。该平台包不是 loader 的依赖，而是**入口包** `node-addon-require-builtin` 的 `optionalDependencies`，pnpm 只把它链接在入口包旁边：

   ```
   <入口>/node_modules/{node-addon-require-builtin, node-addon-native-custom-loader, ...-linux-x64-gnu}
   <loader>/node_modules/{node-addon-native-custom-loader}   ← 没有平台包
   ```

4. pnpm 对 global virtual store 的 hoisting 兜底是 `NODE_PATH`，但只注入 `pnpm run` / `pnpm exec` / 生命周期脚本 / dlx；而 `~/.local/share/pnpm/bin/dsh` 这个 shim 只是一句 `exec node .../lib/bin.js`，没有 `NODE_PATH`，从 shell 启动的 node 拿不到。
5. 最后一道回退 `build/{nodeabi,napi}/<abi>/require_builtin.node` 不存在——该包没有 install/build 脚本，只发 prebuilt，无法现场编译 → 抛错。

#### 2.2 一句话表述

> 该 loader 依赖"未声明依赖的 hoisting"，而 global virtual store 无法向 shell 启动的进程提供 hoisting。

注意这不是"pnpm 的隔离布局整体不兼容 dsh"：同一份 rc.5 安装里，`cordis-plugin-loader` 自己声明的依赖（cordis、cosmokit、schemastery、node-addon-require-builtin）都以兄弟链接存在、解析正常。断掉的只有 loader 未声明、纯靠 hoisting 才能找到的那个平台包。

#### 2.3 "到底更新了哪个软件"：pnpm，不是 dsh，也不是 Node

- **不是 dsh 小版本 bug**：该原生加载器自 dsh 0.1.6 起引入（替代 `--expose-internals`），0.2.1-rc.2 / rc.4 / rc.5 都内置同一个 loader，降级无效；再往下降到 0.1.5 是**另一个**问题（旧版本依赖声明缺陷 → 插件解析失败）。
- **不是 Node 版本**：受支持版本与 v27 alpha 表现完全一致，且失败发生在选 ABI 之前。
- **是 pnpm 的全局布局变更**：同机 9 月的 pnpm 12.2.1 全局安装，`.modules.yaml` 仍是 `virtualStoreDir: .pnpm`（project 布局）；12.16 起的 `pnpm add -g` 改用 global virtual store，于是断链。

### 3. 分析过程

排查的价值在于"每一步只排除一个假设"，且每条都留有可复现的证据。

#### 3.1 先确认安装是否真的坏了

读 `<全局安装>/node_modules/.modules.yaml`：`packageManager`、`virtualStoreDir`、`skipped` 三项；再直接检查 prebuilt 文件是否在位（`...-linux-x64-gnu/0.1.9/<hash>/prebuilt/linux-x64-gnu-napi-v9.node`）。

结论：包与二进制都正常，且被 hoist 到了 `<安装>/node_modules/.pnpm/node_modules`。**安装没问题，问题在加载。**

#### 3.2 定位断裂点：打印 loader 的解析候选路径

从 loader 的真实路径（store 下）调用 `Module._nodeModulePaths()`，把候选目录逐条打出来——一路到 `/node_modules` **都不含** `node-addon-require-builtin-linux-x64-gnu`。这就是"物理路径断裂"的直接证据，而不是猜测。

#### 3.3 验证因果：只补一个变量

在 loader 真实路径上复现 `require('node-addon-require-builtin-linux-x64-gnu')`，得到与 CLI 完全一致的报错（`optional-package` + 两个 `local-build` 失败）；然后**只加** `NODE_PATH=<安装目录>/.pnpm/node_modules`，同一条链立刻成功（`bindingSource=optional-package`、`abi=napi-v9`）。

单变量对照成立 → 断链就是根因，而不是若干失败中的巧合。

#### 3.4 干净环境复现 + 反向验证

临时 `PNPM_HOME` + 临时 store，只装 `node-addon-require-builtin@0.1.9`：

- 默认布局（global virtual store）→ 必现同样的 `No usable native binding found`；
- `PNPM_CONFIG_VIRTUAL_STORE_TYPE=project` → 虚拟存储回到 `node_modules/.pnpm`，`require` 成功。

这既排除了"本机安装被搞坏了"的可能，也把修复方案坐实了。

#### 3.5 排除矩阵（试过但无效的路）

| 尝试 | 结果 | 结论 |
|---|---|---|
| 回退 Node 到受支持版本 | 错误不变 | 失败在选 ABI 之前，与 Node 版本无关 |
| `pnpm approve-builds [-g]` | 无待批准包 | 该包 `package.json` 只有 `build:js`，无 install 脚本 |
| 手配 `allowBuilds: node-addon-require-builtin: true` | 显示 "Already up to date"，错误不变 | 无可构建物，且非根因 |
| `rm -rf node_modules` + `pnpm install` | 错误不变 | 只重建 profile 工作区，不改全局 CLI 拓扑 |
| `pnpm rebuild` | 无效 | 同上，且没有可重建的东西 |
| `node --expose-internals <bin.js> web` | 错误不变 | 附加组件在加载阶段就失败，标志来不及生效 |
| 降级 dsh 到 0.2.1-rc.2 / rc.4 | 同样报原生绑定错误 | 这些版本都内置同一原生加载器 |
| 降级 dsh 到 0.1.5-rc.2 / rc.3 | 绑定错误消失，改为大规模插件解析失败 | 换了另一个问题，不是同一根因 |

#### 3.6 分析过程的两个易错点

- **别把 `pnpm install` 的"成功"当作结论**：它成功只能说明包装对了地方，不能说明能被加载。
- **别被 ABI 字样带偏**：只要报错发生在 `noUsableBindingError`，就说明连"候选二进制"都没进入判断，先怀疑解析，再怀疑版本。

### 4. 缓解方案

按可靠性排序，前两个都经过实测。

#### 4.1 推荐：把全局安装的虚拟存储改回 project 布局

```sh
pnpm config set --global virtualStoreType project
pnpm add -g @deepseek-ai/dsh
```

`pnpm config set --global` 写入 `~/.config/pnpm/config.yaml`，之后 `pnpm add -g` 会读取它，虚拟存储落回 `<安装目录>/node_modules/.pnpm`：包的真实路径重新落在安装目录内，hoisting 目录重回祖先链，Node 原生回溯即可命中，不再依赖 `NODE_PATH` 注入。

- **影响面**：只影响全局安装；profile 内的安装本来就是 project 布局，不受影响。
- **代价**：全局 CLI 失去跨安装/跨版本共享（同一依赖图不再全机一份），磁盘占用略增。对全局 CLI 本就该以"能跑"为先。
- **验证方式**：`grep virtualStoreDir ~/.local/share/pnpm/global/v11/*/node_modules/.modules.yaml`，全部为 `.pnpm`（而不是 `../../../../store/v11/links`）即生效。

#### 4.2 临时：给这次启动补 `NODE_PATH`

```sh
NODE_PATH=$(ls -d ~/.local/share/pnpm/global/v11/*/node_modules/.pnpm/node_modules | head -1) dsh web
```

不改布局、不改配置，适合"先救活一次"。缺点：hash 目录随重装变化，不能硬编码；每条命令都要带。

#### 4.3 退路：改用 npm 全局安装

```sh
npm i -g @deepseek-ai/dsh
```

npm 写扁平 `node_modules`，未声明依赖靠真实目录回溯即可命中，不依赖 `NODE_PATH`，也没有 symlink 指向 store 的路径漂移。代价是全局命令与 pnpm 生态分离。

#### 4.4 上游正解（需 dsh 侧修）

问题在 loader 的解析方式，不在 pnpm。两种改法：

```js
// 相对入口包解析，而不是裸 require
require.resolve(optionalPackageName(packagePrefix), { paths: [packageDir] })
```

或把各平台包也声明为 loader 自己的 `optionalDependencies`。这样 npm、pnpm isolated、global virtual store、yarn 都能解析，不再依赖 hoisting，用户也不必改全局配置。

### 5. `virtualStoreType`：只决定"虚拟存储放哪"

`virtualStoreType` 回答一个问题：虚拟存储是每个项目一份，还是整机一份。取值 `project` / `global`，另有一个旧布尔拼写 `enableGlobalVirtualStore`。它不影响装了什么，只影响装在哪、以及 Node 能不能顺着链接找到未声明的依赖。

| | `project`（默认） | `global` |
|---|---|---|
| 虚拟存储位置 | `<项目>/node_modules/.pnpm` | `<store>/links`（或 `globalVirtualStoreDir`） |
| 目录命名 | `<pkg>@<ver>/node_modules/<pkg>` | `<pkg>/<ver>/<依赖图哈希>/node_modules/<pkg>` |
| 跨项目去重 | 无（文件硬链接共享，目录结构每个项目一份） | 有（同一依赖图全机一份） |
| hoisted 依赖位置 | 项目 `.pnpm/node_modules`，**在真实路径的祖先链上** | 项目 `.pnpm/node_modules`，但**不在** store 真实路径的祖先链上 |
| hoisting 如何送达 | Node 原生回溯就能命中 | 靠 `NODE_PATH` + ESM resolve hook，**只注入 pnpm 自己 spawn 的进程** |
| 冷装 / 热装 | 一般 / 一般 | 冷装更慢（无缓存可复用）/ 多项目热装更快 |
| CI | 友好 | 不友好（缓存通常为空） |
| 默认用于 | 所有项目安装 | **`pnpm add -g`（12.16.0 起）**；`pnpm dlx` 不用；项目安装需显式开启（实验性） |

两类布局的形态差异（同一台机器上可直接对照）：

```
# project 布局
<项目>/node_modules/            lodash -> .pnpm/lodash@4.17.21/node_modules/lodash
<项目>/node_modules/.pnpm/      ← 实体：硬链接自 CAS，是模块解析的祖先目录

# global 布局
<项目>/node_modules/            lodash -> <store>/links/@/lodash/4.17.21/<hash>/node_modules/lodash
<store>/links/.../lodash/       ← 实体在这里，项目内的 .pnpm 不再是祖先
```

## 延伸

### 与 `nodeLinker` 的关系：正交，别混

- `nodeLinker: isolated`（默认）：有虚拟存储，`virtualStoreType` 才有意义。
- `nodeLinker: hoisted`：扁平 `node_modules`，**根本不写虚拟存储**，`virtualStoreType` 对它无效；它走 npm 风格 hoisting，未声明依赖同样能解析——这是另一种解法，代价是失去隔离。
- `pnp` / `loaded`（实验）：也可与两种 store 类型组合。

### 容易和 `virtualStoreType` 搞混的旋钮

| 设置 | 作用 |
|---|---|
| `extendNodePath` | 控制是否注入 `NODE_PATH`；设为 `false` 会整体关掉 global 布局的 hoisting 兜底 |
| `hoistPattern` / `publicHoistPattern` | 决定哪些依赖被提升到 `node_modules`，直接决定"未声明依赖"能否解析 |
| `packageExtensions` | 给漏声明依赖的包补声明（官方推荐的替代方案） |
| `dedupeDirectDeps` | 只影响链接方式，不改变可达性 |

## Q&A

**Q：为什么不干脆降级 dsh？**
A：0.1.6 起内置同一原生加载器，降级到 0.1.5 只是把"原生绑定失败"换成"插件解析失败"，两个问题都源于同一布局假设，治不了根。

**Q：`--expose-internals` 为什么救不了？**
A：附加组件在**加载阶段**就失败，还没轮到访问 Node 内部模块；问题出在模块解析，不是访问权限。

**Q：为什么 npm 全局安装能跑？**
A：npm 写扁平 `node_modules`，未声明依赖靠真实目录回溯即可命中，不依赖 `NODE_PATH` 注入，也不存在 symlink 指向 store 的路径漂移。

**Q：改成 `virtualStoreType project` 会不会影响已经在用的 profile？**
A：不会。该设置只作用于 `pnpm add -g`；profile 内是项目安装，本来就是 project 布局。

**Q：以后怎么避免再踩？**
A：把"全局 CLI 依赖 hoisting"当成已知风险：升级 pnpm 大版本后先跑一次 `<cli> --version`，失败就先查 `.modules.yaml` 的 `virtualStoreDir`。如果该 CLI 长期用 pnpm 全局装，直接固定 `virtualStoreType: project`。

## 参考资料

- pnpm 文档 Global virtual store：<https://pnpm.io/global-virtual-store>（访问 2026-11-11）
- pnpm 文档 Settings / `nodeLinker`、`virtualStoreType`、`hoistPattern`：<https://pnpm.io/settings>（访问 2026-11-11）
- pnpm 文档 `packageExtensions`：<https://pnpm.io/package_json#pnpmpackageextensions>（访问 2026-11-11）
